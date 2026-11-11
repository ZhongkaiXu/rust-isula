use std::io::Write;

use crate::cli::{parse_bool, CliError, Invocation};
use crate::grpc::{logs as rpc, proto};

fn error(message: impl ToString) -> CliError {
    CliError {
        message: message.to_string(),
        exit_code: 1,
    }
}

// Native strtoll(base=0); malformed or out-of-range values mean all logs.
fn tail(value: &str) -> i64 {
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (negative, digits) = match value.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let (radix, digits) = if let Some(digits) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        (16, digits)
    } else if digits.starts_with('0') {
        (8, digits)
    } else {
        (10, digits)
    };
    if digits.is_empty() || !digits.chars().all(|ch| ch.is_digit(radix)) {
        return -1;
    }
    i128::from_str_radix(digits, radix)
        .ok()
        .and_then(|n| i64::try_from(if negative { -n } else { n }).ok())
        .unwrap_or(-1)
}

pub fn build(command: &Invocation) -> Result<proto::LogsRequest, CliError> {
    if command.args.len() != 1 {
        return Err(error("logs requires exactly one container"));
    }
    let mut request = proto::LogsRequest {
        id: command.args[0].clone(),
        tail: -1,
        ..Default::default()
    };
    for option in &command.options {
        match option.name {
            "follow" => request.follow = parse_bool(option.value.as_deref(), "follow")?,
            "timestamps" => request.timestamps = parse_bool(option.value.as_deref(), "timestamps")?,
            "tail" => request.tail = tail(option.value.as_deref().unwrap_or_default()),
            _ => {}
        }
    }
    Ok(request)
}

fn status(name: &str, status: tonic::Status) -> Result<(), CliError> {
    if status.message().starts_with("disable console log") {
        writeln!(
            std::io::stdout(),
            "[WARNING]: Container {name} disable console log!"
        )
        .map_err(error)
    } else {
        Err(error(status))
    }
}

pub async fn run(command: &Invocation) -> Result<(), CliError> {
    let request = build(command)?;
    let timestamps = request.timestamps;
    let mut stream = match rpc::logs(&command.host, request).await {
        Ok(stream) => stream,
        Err(err) => return status(&command.args[0], err),
    };
    loop {
        let frame = match stream.message().await {
            Ok(Some(frame)) => frame,
            Ok(None) => return Ok(()),
            Err(err) => return status(&command.args[0], err),
        };
        let mut stdout = std::io::stdout();
        let mut stderr = std::io::stderr();
        let output: &mut dyn Write = match frame.stream.as_str() {
            "stdout" => &mut stdout,
            "stderr" => &mut stderr,
            _ => {
                eprintln!("Invalid container log: {}", frame.stream);
                continue;
            }
        };
        if timestamps {
            write!(output, "{} ", frame.time).map_err(error)?;
        }
        output
            .write_all(&frame.data)
            .and_then(|()| output.flush())
            .map_err(error)?;
    }
}

#[cfg(test)]
mod tests {
    use super::tail;

    #[test]
    fn tail_matches_native_integer_conversion_and_fallback() {
        for (text, expected) in [
            ("0", 0),
            ("010", 8),
            ("0x10", 16),
            (" +12", 12),
            ("-2", -2),
            ("-0x8000000000000000", i64::MIN),
            ("9223372036854775807", i64::MAX),
            ("9223372036854775808", -1),
            ("08", -1),
            ("all", -1),
            ("", -1),
            ("2 ", -1),
            ("--2", -1),
        ] {
            assert_eq!(tail(text), expected, "{text}");
        }
    }
}
