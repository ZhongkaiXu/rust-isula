use crate::cli::{CliError, Invocation};
use crate::grpc;

// SIGNAL_MAP_DEFAULT from iSulad, including its Linux compatibility aliases.
const SIGNALS: &[(&str, i32)] = &[
    ("HUP", libc::SIGHUP),
    ("INT", libc::SIGINT),
    ("QUIT", libc::SIGQUIT),
    ("ILL", libc::SIGILL),
    ("TRAP", libc::SIGTRAP),
    ("ABRT", libc::SIGABRT),
    ("IOT", libc::SIGABRT),
    ("BUS", libc::SIGBUS),
    ("EMT", 7),
    ("FPE", libc::SIGFPE),
    ("KILL", libc::SIGKILL),
    ("USR1", libc::SIGUSR1),
    ("SEGV", libc::SIGSEGV),
    ("USR2", libc::SIGUSR2),
    ("PIPE", libc::SIGPIPE),
    ("ALRM", libc::SIGALRM),
    ("TERM", libc::SIGTERM),
    ("STKFLT", libc::SIGSTKFLT),
    ("CHLD", libc::SIGCHLD),
    ("CLD", libc::SIGCHLD),
    ("CONT", libc::SIGCONT),
    ("STOP", libc::SIGSTOP),
    ("TSTP", libc::SIGTSTP),
    ("TTIN", libc::SIGTTIN),
    ("TTOU", libc::SIGTTOU),
    ("URG", libc::SIGURG),
    ("XCPU", libc::SIGXCPU),
    ("XFSZ", libc::SIGXFSZ),
    ("VTALRM", libc::SIGVTALRM),
    ("PROF", libc::SIGPROF),
    ("WINCH", libc::SIGWINCH),
    ("IO", libc::SIGIO),
    ("POLL", libc::SIGIO),
    ("INFO", 29),
    ("LOST", 37),
    ("PWR", libc::SIGPWR),
    ("UNUSED", 31),
    ("SYS", libc::SIGSYS),
];

fn realtime_offset(name: &str, prefix: &str, max: i32) -> Option<i32> {
    let text = name.strip_prefix(prefix)?;
    let offset = text.parse::<i32>().ok()?;
    // The upstream table only contains canonical names, without leading zeros.
    ((1..=max).contains(&offset) && text == offset.to_string()).then_some(offset)
}

fn parse_signal(value: &str) -> Result<u32, String> {
    let invalid = || format!("invalid signal: {value}");
    let rtmin = libc::SIGRTMIN();
    let rtmax = libc::SIGRTMAX();
    let signal = if value.starts_with(|ch: char| ch.is_ascii_digit()) {
        let (radix, digits) = if let Some(digits) = value
            .strip_prefix("0x")
            .or_else(|| value.strip_prefix("0X"))
        {
            (16, digits)
        } else if value.starts_with('0') {
            (8, value)
        } else {
            (10, value)
        };
        if digits.is_empty() || !digits.chars().all(|ch| ch.is_digit(radix)) {
            return Err(invalid());
        }
        i32::from_str_radix(digits, radix).map_err(|_| invalid())?
    } else {
        let upper = value.to_ascii_uppercase();
        let name = upper.strip_prefix("SIG").unwrap_or(&upper);
        if let Some((_, signal)) = SIGNALS.iter().find(|(known, _)| *known == name) {
            *signal
        } else if name == "RTMIN" {
            rtmin
        } else if name == "RTMAX" {
            rtmax
        } else if let Some(offset) = realtime_offset(name, "RTMIN+", 15) {
            rtmin + offset
        } else if let Some(offset) = realtime_offset(name, "RTMAX-", 14) {
            rtmax - offset
        } else {
            return Err(invalid());
        }
    };
    if SIGNALS.iter().any(|(_, known)| *known == signal)
        || (rtmin..=rtmin + 15).contains(&signal)
        || (rtmax - 14..=rtmax).contains(&signal)
    {
        Ok(signal as u32)
    } else {
        Err(format!("the Linux daemon does not support signal {signal}"))
    }
}

pub async fn run(command: &Invocation) -> Result<bool, CliError> {
    // Upstream validates only the final signal, before checking container count.
    let value = command
        .options
        .iter()
        .rev()
        .find(|option| option.name == "signal")
        .and_then(|option| option.value.as_deref())
        .unwrap_or("SIGKILL");
    let signal = parse_signal(value).map_err(|message| CliError {
        message,
        exit_code: 1,
    })?;
    if command.args.is_empty() || command.args.len() >= 1000 {
        return Err(CliError {
            message: "kill requires between 1 and 999 container names".into(),
            exit_code: 125,
        });
    }

    let mut success = true;
    for name in &command.args {
        if name.is_empty() {
            eprintln!("risula kill: container name cannot be empty");
            success = false;
            continue;
        }
        match grpc::kill_container(&command.host, name, signal).await {
            Ok(()) => println!("{name}"),
            Err(error) => {
                eprintln!("risula kill: {name}: {error}");
                success = false;
            }
        }
    }
    Ok(success)
}
