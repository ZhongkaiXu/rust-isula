use std::time::Duration;

use super::console::{Input, Terminal};
use crate::cli::{CliError, Invocation};
use crate::grpc::{attach as rpc, start::wait_container};

fn error(message: impl ToString) -> CliError {
    CliError {
        message: message.to_string(),
        exit_code: 1,
    }
}

pub async fn run(command: &Invocation) -> Result<i32, CliError> {
    if command.args.len() != 1 || command.args[0].is_empty() {
        return Err(error("attach requires exactly one container"));
    }
    let (id, tty) = rpc::inspect(&command.host, &command.args[0])
        .await
        .map_err(error)?;
    if tty && unsafe { libc::isatty(0) } != 1 {
        return Err(error("the input device is not a TTY"));
    }
    let _terminal = Terminal::new(tty).map_err(error)?;
    let input = Input::new().map_err(error)?;
    // The native client starts Wait alongside Attach, then allows one second
    // after the stream closes for the container's exit status to arrive.
    let wait = async {
        match wait_container(&command.host, &id).await {
            Ok(code) => code as i32,
            Err(_) => std::future::pending().await,
        }
    };
    let attach = rpc::attach(&command.host, &id, input);
    tokio::pin!(wait, attach);
    let mut exit_code = None;
    let attached = tokio::select! {
        biased;
        code = &mut wait => {
            exit_code = Some(code);
            attach.await
        }
        result = &mut attach => result,
    };
    if let Err(message) = attached {
        let mut failure = error(message);
        if let Some(code) = exit_code.filter(|code| *code != 0) {
            failure.exit_code = code;
        }
        return Err(failure);
    }
    match exit_code {
        Some(code) => Ok(code),
        None => tokio::time::timeout(Duration::from_secs(1), wait)
            .await
            .map_err(|_| error("timed out waiting for container exit status")),
    }
}
