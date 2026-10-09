use crate::cli::Invocation;
use crate::grpc;

// Match the upstream strtol(base=0): signed decimal, octal or hexadecimal i32.
fn parse_time(value: &str) -> Result<i32, String> {
    let text = value.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (negative, digits) = if let Some(rest) = text.strip_prefix('-') {
        (true, rest)
    } else {
        (false, text.strip_prefix('+').unwrap_or(text))
    };
    let (radix, digits) = if let Some(rest) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        (16, rest)
    } else if digits.starts_with('0') {
        (8, digits)
    } else {
        (10, digits)
    };
    let invalid = || format!("invalid value '{value}' for --time: expected a 32-bit integer");
    if digits.is_empty() || !digits.chars().all(|ch| ch.is_digit(radix)) {
        return Err(invalid());
    }
    let magnitude = i64::from_str_radix(digits, radix).map_err(|_| invalid())?;
    i32::try_from(if negative { -magnitude } else { magnitude }).map_err(|_| invalid())
}

// Err is an invalid option; Ok(false) means at least one container failed.
pub async fn run(command: &Invocation) -> Result<bool, String> {
    let mut timeout = 10;
    let mut force = false;
    for option in &command.options {
        match option.name {
            "time" => timeout = parse_time(option.value.as_deref().unwrap_or(""))?,
            "force" => {
                force = match option.value.as_deref().unwrap_or("true") {
                    "1" | "t" | "T" | "true" | "TRUE" | "True" => true,
                    "0" | "f" | "F" | "false" | "FALSE" | "False" => false,
                    value => return Err(format!("invalid boolean value '{value}' for --force")),
                };
            }
            _ => {}
        }
    }
    if force {
        timeout = 0;
    }

    let mut success = true;
    for name in &command.args {
        if name.is_empty() {
            eprintln!("risula stop: container name cannot be empty");
            success = false;
            continue;
        }
        match grpc::stop_container(&command.host, name, force, timeout).await {
            Ok(response) => {
                if !response.errmsg.is_empty() {
                    eprintln!("{}", response.errmsg);
                }
                println!("{name}");
            }
            Err(error) => {
                eprintln!("risula stop: {name}: {error}");
                success = false;
            }
        }
    }
    Ok(success)
}
