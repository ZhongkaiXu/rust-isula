use std::fmt::Write;

use chrono::{DateTime, Utc};

use crate::cli::Invocation;
use crate::grpc::{
    self,
    proto::{Container, ListRequest},
};

const FIELDS: [&str; 14] = [
    "ID",
    "Image",
    "Command",
    "Created",
    "Status",
    "Ports",
    "Names",
    "Pid",
    "ExitCode",
    "RestartCount",
    "StartAt",
    "FinishAt",
    "Runtime",
    "State",
];
const HEADERS: [&str; 14] = [
    "CONTAINER ID",
    "IMAGE",
    "COMMAND",
    "CREATED",
    "STATUS",
    "PORTS",
    "NAMES",
    "PID",
    "EXIT_CODE",
    "RESTART_COUNT",
    "STARTAT",
    "FINISHAT",
    "RUNTIME",
    "STATE",
];
const DEFAULT_FORMAT: &str =
    "table {{.ID}}\t{{.Image}}\t{{.Command}}\t{{.Created}}\t{{.Status}}\t{{.Ports}}\t{{.Names}}";

enum Part {
    Text(String),
    Field(usize),
}
struct Format {
    table: bool,
    parts: Vec<Part>,
}

fn parse_format(source: &str) -> Result<Format, String> {
    let source = if source == "table" {
        DEFAULT_FORMAT
    } else {
        source
    };
    let (table, mut rest) = match source.strip_prefix("table ") {
        Some(rest) => (true, rest),
        None => (false, source),
    };
    let mut parts = Vec::new();
    while let Some(start) = rest.find("{{") {
        let text = &rest[..start];
        if text.contains("}}") {
            return Err("invalid ps format".into());
        }
        parts.push(Part::Text(unescape(text)));
        rest = &rest[start + 2..];
        let end = rest.find("}}").ok_or("invalid ps format")?;
        let name = rest[..end]
            .trim()
            .strip_prefix('.')
            .ok_or("invalid ps format")?;
        let field = FIELDS
            .iter()
            .position(|field| *field == name)
            .ok_or_else(|| format!("unsupported ps field: {name}"))?;
        parts.push(Part::Field(field));
        rest = &rest[end + 2..];
    }
    if rest.contains("}}") || parts.is_empty() {
        return Err("invalid ps format".into());
    }
    parts.push(Part::Text(unescape(rest)));
    Ok(Format { table, parts })
}

fn unescape(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some(next) => {
                    result.push('\\');
                    result.push(next);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn request(command: &Invocation) -> Result<(ListRequest, bool, bool, Format), String> {
    let mut request = ListRequest::default();
    let (mut quiet, mut no_trunc, mut latest) = (false, false, false);
    let mut last = 0;
    let mut format = DEFAULT_FORMAT;
    for option in &command.options {
        let value = option.value.as_deref().unwrap_or("");
        match option.name {
            "all" => request.all = true,
            "quiet" => quiet = true,
            "no-trunc" => no_trunc = true,
            "latest" => latest = true,
            "last" => {
                last = value
                    .parse::<u32>()
                    .map_err(|_| "invalid --last: expected a nonnegative 32-bit integer")?
            }
            "format" => format = value,
            "filter" => {
                let (key, value) = value
                    .split_once('=')
                    .ok_or("invalid filter: expected name=value")?;
                let key = key.trim().to_ascii_lowercase();
                if key.is_empty() {
                    return Err("filter name cannot be empty".into());
                }
                request.filters.insert(key, value.trim().into());
            }
            _ => {}
        }
    }
    if latest {
        last = 1;
    }
    if last > 0 {
        request.filters.clear();
        request.filters.insert("last_n".into(), last.to_string());
    }
    Ok((request, quiet, no_trunc, parse_format(format)?))
}

fn duration(seconds: i64) -> String {
    match seconds {
        s if s <= 0 => "-".into(),
        s if s < 60 => format!("{s} seconds"),
        s if s < 120 => "About a minute".into(),
        s if s < 3600 => format!("{} minutes", s / 60),
        s if s < 7200 => "About an hour".into(),
        s if s < 172800 => format!("{} hours", s / 3600),
        s if s < 1209600 => format!("{} days", s / 86400),
        s if s < 7776000 => format!("{} weeks", s / 604800),
        s if s < 63072000 => format!("{} months", s / 2592000),
        s => format!("{} years", s / 31536000),
    }
}

fn elapsed(timestamp: &str, now: i64, ago: bool) -> String {
    let Ok(time) = DateTime::parse_from_rfc3339(timestamp) else {
        return "-".into();
    };
    if timestamp.starts_with("0001-") {
        return "-".into();
    }
    let text = duration(now - time.timestamp());
    if ago && text != "-" {
        format!("{text} ago")
    } else {
        text
    }
}

fn present<'a>(text: &'a str, empty: &'a str) -> &'a str {
    if text.is_empty() {
        empty
    } else {
        text
    }
}

fn row(c: &Container, no_trunc: bool, now: i64) -> [String; 14] {
    let start = elapsed(&c.startat, now, false);
    let finish = elapsed(&c.finishat, now, true);
    let state = match c.status {
        1 => "inited",
        2 => "starting",
        3 => "running",
        4 => "exited",
        5 => "paused",
        6 => "restarting",
        _ => "unknown",
    };
    let status = match c.status {
        1 => "Created".into(),
        2 => format!("Starting {start}"),
        3 if !c.health_state.is_empty() => format!("Up {start} ({})", c.health_state),
        3 => format!("Up {start}"),
        5 => format!("Up {start} (Paused)"),
        6 => format!("Restarting ({}) {finish}", c.exit_code),
        _ => format!("Exited ({}) {finish}", c.exit_code),
    };
    // Upstream truncates Command even with --no-trunc; quiet always prints full IDs.
    let cmd = present(&c.command, "-");
    let command = if cmd.chars().count() > 20 {
        format!("\"{}...\" ", cmd.chars().take(17).collect::<String>())
    } else {
        format!("\"{cmd}\"")
    };
    [
        if no_trunc {
            c.id.clone()
        } else {
            c.id.chars().take(12).collect()
        },
        present(&c.image, "none").into(),
        command,
        {
            let age = duration(now - c.created / 1_000_000_000);
            if age == "-" {
                age
            } else {
                format!("{age} ago")
            }
        },
        status,
        c.ports.clone(),
        present(&c.name, "-").into(),
        if c.pid == 0 {
            "-".into()
        } else {
            c.pid.to_string()
        },
        c.exit_code.to_string(),
        c.restartcount.to_string(),
        start,
        elapsed(&c.finishat, now, false),
        present(&c.runtime, "lcr").into(),
        state.into(),
    ]
}

fn render(format: &Format, values: &[String; 14], widths: &[usize; 14]) -> String {
    let mut line = String::new();
    for part in &format.parts {
        match part {
            Part::Text(text) => line.push_str(text),
            Part::Field(2) if values[2].chars().count() > 22 => line.push_str(&values[2]),
            Part::Field(i) => write!(line, "{:<width$}", values[*i], width = widths[*i]).unwrap(),
        }
    }
    line
}

pub async fn run(command: &Invocation) -> Result<(), String> {
    let (request, quiet, no_trunc, format) = request(command)?;
    let all = request.all;
    let mut containers = grpc::list_containers(request, &command.host).await?;
    if all {
        containers.sort_by(|a, b| b.startat.cmp(&a.startat));
    }
    if quiet {
        for c in containers {
            println!("{:<12}", c.id);
        }
        return Ok(());
    }
    let now = Utc::now().timestamp();
    let rows: Vec<_> = containers.iter().map(|c| row(c, no_trunc, now)).collect();
    let mut widths = HEADERS.map(str::len);
    for row in &rows {
        for (i, value) in row.iter().enumerate() {
            widths[i] = widths[i].max(value.chars().count());
        }
    }
    // Upstream measures raw command length and time text including " ago".
    widths[2] = containers
        .iter()
        .map(|c| present(&c.command, "-").chars().count() + 2)
        .fold(7, usize::max);
    for c in &containers {
        widths[10] = widths[10].max(elapsed(&c.startat, now, true).len());
        widths[11] = widths[11].max(elapsed(&c.finishat, now, true).len());
    }
    if format.table {
        widths[2] = widths[2].min(22);
    }
    if format.table {
        println!("{}", render(&format, &HEADERS.map(str::to_string), &widths));
    }
    for row in rows {
        println!("{}", render(&format, &row, &widths));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_matches_upstream_boundaries() {
        for (seconds, expected) in [
            (0, "-"),
            (1, "1 seconds"),
            (60, "About a minute"),
            (120, "2 minutes"),
            (3600, "About an hour"),
            (86400, "24 hours"),
            (172800, "2 days"),
            (13 * 86400, "13 days"),
            (14 * 86400, "2 weeks"),
            (90 * 86400, "3 months"),
            (365 * 86400, "12 months"),
            (730 * 86400, "2 years"),
        ] {
            assert_eq!(duration(seconds), expected);
        }
    }

    #[test]
    fn container_states_and_unset_timestamps() {
        let mut c = Container::default();
        for (state, expected) in [
            (1, "Created"),
            (2, "Starting -"),
            (3, "Up -"),
            (4, "Exited (0) -"),
            (5, "Up - (Paused)"),
            (6, "Restarting (0) -"),
        ] {
            c.status = state;
            assert_eq!(row(&c, false, 100)[4], expected);
        }
        assert_eq!(elapsed("0001-01-01T00:00:00Z", 100, true), "-");
        assert_eq!(elapsed("1970-01-01T00:02:00Z", 100, true), "-");
    }
}
