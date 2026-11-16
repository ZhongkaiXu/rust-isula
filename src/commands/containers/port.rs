use std::io::{self, Write};

use serde_json::Value;

use crate::cli::{CliError, Invocation};
use crate::grpc;

fn failed(message: impl ToString) -> CliError {
    CliError {
        message: message.to_string(),
        exit_code: 255,
    }
}

fn number(text: &str) -> Option<u16> {
    let text = text.trim_start_matches([' ', '\t', '\r', '\n', '\x0b', '\x0c']);
    let text = text.strip_prefix('+').unwrap_or(text);
    let (radix, digits) =
        if let Some(digits) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            (16, digits)
        } else if text.starts_with('0') {
            (8, text)
        } else {
            (10, text)
        };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    u16::from_str_radix(digits, radix).ok().filter(|&n| n != 0)
}

fn filter(text: &str) -> Result<String, CliError> {
    // Native split drops empty components and does not validate the protocol.
    let parts: Vec<_> = text.split('/').filter(|part| !part.is_empty()).collect();
    let (ports, protocol) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        (text, "tcp")
    };
    let invalid = || failed(format!("invalid port argument: {text}"));
    let (start, end) = if ports.contains('-') {
        let range: Vec<_> = ports.split('-').filter(|part| !part.is_empty()).collect();
        if range.len() != 2 {
            return Err(invalid());
        }
        (
            number(range[0]).ok_or_else(invalid)?,
            number(range[1]).ok_or_else(invalid)?,
        )
    } else {
        let n = number(ports).ok_or_else(invalid)?;
        (n, n)
    };
    if start > end {
        return Err(invalid());
    }
    Ok(if start == end {
        format!("{start}/{protocol}")
    } else {
        format!("{start}-{end}/{protocol}")
    })
}

fn object(value: &Value) -> Result<&serde_json::Map<String, Value>, CliError> {
    value
        .as_object()
        .ok_or_else(|| failed("invalid port mapping object"))
}

fn output(text: &str, selected: Option<&str>) -> Result<String, CliError> {
    let data: Value = serde_json::from_str(text).map_err(failed)?;
    let root = object(&data)?;
    let mut lines = String::new();
    let Some(settings) = root.get("NetworkSettings").filter(|v| !v.is_null()) else {
        return Ok(lines);
    };
    let Some(ports) = object(settings)?.get("Ports").filter(|v| !v.is_null()) else {
        return Ok(lines);
    };
    for (port, binding) in object(ports)? {
        if binding.is_null() {
            continue;
        }
        let Some(element) = object(binding)?.get("Element").filter(|v| !v.is_null()) else {
            continue;
        };
        let Some(hosts) = object(element)?.get("host").filter(|v| !v.is_null()) else {
            continue;
        };
        for host in hosts
            .as_array()
            .ok_or_else(|| failed("invalid port binding list"))?
        {
            let host = object(host)?;
            let string = |key: &str| -> Result<Option<&str>, CliError> {
                host.get(key)
                    .filter(|v| !v.is_null())
                    .map(|v| {
                        v.as_str()
                            .ok_or_else(|| failed("invalid port binding string"))
                    })
                    .transpose()
            };
            let ip = string("HostIP")?
                .filter(|s| !s.is_empty())
                .unwrap_or("0.0.0.0");
            let host_port = string("HostPort")?.unwrap_or("(null)");
            if selected.is_none_or(|key| key == port) {
                lines.push_str(&format!("{port} -> {ip}:{host_port}\n"));
            }
        }
    }
    Ok(lines)
}

pub async fn run(command: &Invocation) -> Result<(), CliError> {
    if command.args.is_empty() || command.args.len() >= 1000 {
        return Err(CliError {
            message: "port requires between 1 and 999 arguments".into(),
            exit_code: 125,
        });
    }
    let selected = command.args.get(1).map(|s| filter(s)).transpose()?;
    if command.args[0].is_empty() {
        return Err(failed("Missing container name in the request"));
    }
    let json = grpc::port::inspect(&command.host, &command.args[0])
        .await
        .map_err(failed)?;
    io::stdout()
        .lock()
        .write_all(output(&json, selected.as_deref())?.as_bytes())
        .map_err(failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_filter_normalization() {
        for (input, expected) in [
            ("80", "80/tcp"),
            ("010", "8/tcp"),
            ("0x50/udp", "80/udp"),
            (" +80/TCP", "80/TCP"),
            ("80-80", "80/tcp"),
            ("80--81", "80-81/tcp"),
            ("/80//custom/", "80/custom"),
        ] {
            assert_eq!(filter(input).unwrap(), expected);
        }
        for input in [
            "",
            "0",
            "65536",
            "08",
            "-80",
            "80/",
            "80/tcp/udp",
            "81-80",
            "80 ",
            "999999999999999999999999",
        ] {
            assert!(filter(input).is_err(), "{input}");
        }
    }
}
