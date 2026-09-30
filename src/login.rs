use std::io::{self, IsTerminal, Read, Write};

use crate::cli::Invocation;
use crate::grpc;

const MAX_CREDENTIAL_LEN: usize = 255;

fn option<'a>(command: &'a Invocation, name: &str) -> Option<&'a str> {
    command
        .options
        .iter()
        .rev()
        .find(|option| option.name == name)
        .and_then(|option| option.value.as_deref())
}

fn read_password_stdin() -> Result<String, String> {
    let mut password = String::new();
    io::stdin()
        .take((MAX_CREDENTIAL_LEN + 3) as u64)
        .read_to_string(&mut password)
        .map_err(|error| format!("cannot read password from stdin: {error}"))?;
    if password.ends_with('\n') {
        password.pop();
    }
    if password.ends_with('\r') {
        password.pop();
    }
    if password.is_empty() || password.len() > MAX_CREDENTIAL_LEN {
        return Err("password must be between 1 and 255 bytes".to_string());
    }
    Ok(password)
}

struct EchoGuard(libc::termios);

impl EchoGuard {
    fn hide() -> io::Result<Self> {
        let mut original = unsafe { std::mem::zeroed::<libc::termios>() };
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut original) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut hidden = original;
        hidden.c_lflag &= !libc::ECHO;
        if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &hidden) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(original))
    }
}

impl Drop for EchoGuard {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.0) };
    }
}

fn read_terminal_line(prompt: &str, hide: bool) -> Result<String, String> {
    if !io::stdin().is_terminal() {
        return Err("cannot perform an interactive login from a non TTY device".to_string());
    }
    print!("{prompt}: ");
    io::stdout()
        .flush()
        .map_err(|error| format!("cannot show {prompt} prompt: {error}"))?;

    let mut line = String::new();
    let bytes = if hide {
        let guard =
            EchoGuard::hide().map_err(|error| format!("cannot disable terminal echo: {error}"))?;
        let result = io::stdin().read_line(&mut line);
        drop(guard);
        println!();
        result
    } else {
        io::stdin().read_line(&mut line)
    }
    .map_err(|error| format!("cannot read {prompt}: {error}"))?;

    if bytes == 0 {
        return Err(format!("{prompt} is required"));
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

pub async fn run(command: &Invocation) -> Result<(), String> {
    if !command.host.starts_with("unix://") {
        return Err("login requires a Unix socket; TCP credentials are not encrypted".to_string());
    }

    let username_option = option(command, "username");
    let password_option = option(command, "password");
    let password_stdin = command
        .options
        .iter()
        .any(|option| option.name == "password-stdin");

    if password_stdin && password_option.is_some() {
        return Err("--password and --password-stdin are mutually exclusive".to_string());
    }
    if password_stdin && username_option.is_none() {
        return Err("--password-stdin requires --username".to_string());
    }

    let username = match username_option {
        Some(value) => value.to_string(),
        None => read_terminal_line("Username", false)?,
    };
    if username.is_empty() || username.len() > MAX_CREDENTIAL_LEN {
        return Err("username must be between 1 and 255 bytes".to_string());
    }

    let password = if password_stdin {
        read_password_stdin()?
    } else if let Some(value) = password_option {
        println!("WARNING! Using --password via the CLI is insecure. Use --password-stdin.");
        value.to_string()
    } else {
        read_terminal_line("Password", true)?
    };
    if password.is_empty() || password.len() > MAX_CREDENTIAL_LEN {
        return Err("password must be between 1 and 255 bytes".to_string());
    }

    grpc::login(&command.host, &command.args[0], &username, &password).await?;
    eprintln!("Login Succeeded");
    Ok(())
}
