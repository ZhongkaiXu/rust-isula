use std::io::Write;

use chrono::{Datelike, NaiveDate, TimeZone};
use prost_types::Timestamp;

use crate::cli::{CliError, Invocation};
use crate::grpc::{events as rpc, proto};

fn error(message: impl ToString) -> CliError {
    CliError {
        message: message.to_string(),
        exit_code: 1,
    }
}

// Match util_get_tm_from_str: select by length, then read numeric fields
// separated by one byte. Native events accepts local dates, not Unix epochs.
fn timestamp(text: &str) -> Option<Timestamp> {
    let bytes = text.as_bytes();
    let nanos = if bytes.contains(&b'.') {
        if !(21..=29).contains(&bytes.len()) {
            return None;
        }
        let mut nanos = 0i32;
        for &ch in &bytes[20..] {
            nanos = nanos.wrapping_mul(10).wrapping_add(i32::from(ch) - 48);
        }
        for _ in bytes.len()..29 {
            nanos = nanos.wrapping_mul(10);
        }
        nanos
    } else {
        let length = if bytes.contains(&b'T') {
            match bytes.iter().filter(|&&ch| ch == b':').count() {
                0 => 13,
                1 => 16,
                2 => 19,
                _ => return None,
            }
        } else {
            10
        };
        if bytes.len() != length {
            return None;
        }
        0
    };
    let mut fields = [0i32; 6];
    let mut pos = 0;
    for field in &mut fields {
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            *field = field
                .checked_mul(10)?
                .checked_add(i32::from(bytes[pos] - b'0'))?;
            pos += 1;
        }
        if pos == bytes.len() {
            break;
        }
        pos += 1;
    }
    let [year, month, day, hour, minute, second] = fields;
    if !(1900..=2100).contains(&year) {
        return None;
    }
    let date = NaiveDate::from_ymd_opt(year, month as u32, day as u32)?;
    date.and_hms_opt(hour as u32, minute as u32, second as u32)?;
    // Native uses the current DST flag even when parsing a historical date.
    let seconds = unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut current: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut current).is_null() {
            return None;
        }
        let mut tm: libc::tm = std::mem::zeroed();
        tm.tm_year = date.year() - 1900;
        tm.tm_mon = month - 1;
        tm.tm_mday = day;
        tm.tm_hour = hour;
        tm.tm_min = minute;
        tm.tm_sec = second;
        tm.tm_isdst = current.tm_isdst;
        libc::mktime(&mut tm)
    };
    Some(Timestamp { seconds, nanos })
}

pub fn build(command: &Invocation) -> Result<proto::EventsRequest, CliError> {
    let mut request = proto::EventsRequest::default();
    // The native command ignores positional arguments and uses the last option.
    for name in ["name", "since", "until"] {
        if let Some(value) = command
            .options
            .iter()
            .rev()
            .find(|o| o.name == name)
            .and_then(|o| o.value.as_deref())
        {
            match name {
                "name" => request.id = value.to_owned(),
                "since" => {
                    request.since = Some(
                        timestamp(value).ok_or_else(|| error("Failed to get since timestamp"))?,
                    )
                }
                _ => {
                    request.until = Some(
                        timestamp(value).ok_or_else(|| error("Failed to get until timestamp"))?,
                    )
                }
            }
        }
    }
    Ok(request)
}

fn format_event(event: proto::Event) -> String {
    let time = event
        .timestamp
        .filter(|t| t.seconds != 0)
        .and_then(|t| {
            chrono::Local
                .timestamp_opt(t.seconds, 0)
                .single()
                .map(|date| {
                    format!(
                        "{}.{:09}{}",
                        date.format("%Y-%m-%dT%H:%M:%S"),
                        t.nanos,
                        date.format("%:z")
                    )
                })
        })
        .unwrap_or_else(|| "-".into());
    let mut line = format!("{time} {} {}", event.opt, event.id);
    if !event.annotations.is_empty() {
        let mut annotations: Vec<_> = event
            .annotations
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        annotations.sort();
        line.push_str(&format!(" ({})", annotations.join(", ")));
    }
    line
}

pub async fn run(command: &Invocation) -> Result<(), CliError> {
    let mut stream = rpc::events(&command.host, build(command)?)
        .await
        .map_err(error)?;
    while let Some(event) = stream.message().await.map_err(error)? {
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{}", format_event(event))
            .and_then(|()| stdout.flush())
            .map_err(error)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_dates_and_fractional_seconds() {
        for text in [
            "2024-02-29",
            "2024-02-29T12",
            "2024-02-29T12:34",
            "2024-02-29T12:34:56",
        ] {
            assert_eq!(timestamp(text).unwrap().nanos, 0);
        }
        assert_eq!(
            timestamp("2024-02-29T12:34:56.123").unwrap().nanos,
            123000000
        );
        for text in [
            "2023-02-29",
            "1899-01-01",
            "2101-01-01",
            "1h",
            "1700000000",
            "2024-01-01T24:00:00",
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00.",
            "2024-01-01T00:00:00.1234567890",
        ] {
            assert!(timestamp(text).is_none(), "{text}");
        }
    }
    #[test]
    fn missing_timestamp_and_annotations() {
        let event = proto::Event {
            opt: "container start".into(),
            id: "abc".into(),
            annotations: [
                ("name".into(), "demo".into()),
                ("image".into(), "busybox".into()),
            ]
            .into(),
            ..Default::default()
        };
        assert_eq!(
            format_event(event),
            "- container start abc (image=busybox, name=demo)"
        );
        assert_eq!(
            format_event(proto::Event {
                timestamp: Some(Timestamp {
                    seconds: 0,
                    nanos: 1
                }),
                ..Default::default()
            }),
            "-  "
        );
    }
}
