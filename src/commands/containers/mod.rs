pub(crate) mod console;
pub mod create;
pub mod exec;
pub mod inspect;
pub mod kill;
pub mod pause;
pub mod ps;
pub mod rename;
pub mod restart;
pub mod rm;
pub mod run;
pub mod start;
pub mod stats;
pub mod stop;
pub mod top;
pub mod unpause;
pub mod wait;

// Match the upstream strtol(base=0): signed decimal, octal or hexadecimal i32.
pub(super) fn parse_time(value: &str) -> Result<i32, String> {
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
