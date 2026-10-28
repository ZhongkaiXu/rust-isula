use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use tokio::io::unix::AsyncFd;

use super::console::{self, Input};
use crate::cli::{parse_bool, CliError, Invocation};
use crate::grpc::{proto, start as rpc};

fn error(message: impl ToString, exit_code: i32) -> CliError {
    CliError {
        message: message.to_string(),
        exit_code,
    }
}

pub(super) fn start_result(
    result: Result<proto::StartResponse, tonic::Status>,
    remote: bool,
) -> Result<(), CliError> {
    let response = result.map_err(|status| {
        let code = match status.code() {
            tonic::Code::Unknown | tonic::Code::PermissionDenied | tonic::Code::Internal => 1,
            _ => 125,
        };
        error(status, code)
    })?;
    if response.cc == 0 {
        return Ok(());
    }
    let message = response.errmsg.to_ascii_lowercase();
    let code = if !remote
        && [
            "executable file not found",
            "no such file or directory",
            "system cannot find the file specified",
        ]
        .iter()
        .any(|text| message.contains(text))
    {
        127
    } else if !remote && message.contains("permission denied") {
        126
    } else if !remote && message.contains("not a directory") {
        127
    } else {
        125
    };
    Err(error(response.errmsg, code))
}

// The private temporary directory owns both FIFOs and removes them on every return path.
fn open_fifo(path: &Path) -> std::io::Result<(AsyncFd<File>, File)> {
    let name = CString::new(path.as_os_str().as_encoded_bytes())?;
    // SAFETY: name is NUL-terminated and remains alive for the syscall.
    if unsafe { libc::mkfifo(name.as_ptr(), 0o600) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let reader = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)?;
    // Hold a writer until Start returns so a fast container cannot cause premature EOF.
    let writer = OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)?;
    Ok((AsyncFd::new(reader)?, writer))
}

async fn copy_output(fifo: AsyncFd<File>, mut output: impl Write) -> Result<(), CliError> {
    let mut buffer = [0; 16 * 1024];
    loop {
        let mut ready = fifo.readable().await.map_err(|err| error(err, 1))?;
        let result = ready.try_io(|fd| fd.get_ref().read(&mut buffer));
        match result {
            Ok(Ok(0)) => return Ok(()),
            Ok(Ok(size)) => output
                .write_all(&buffer[..size])
                .and_then(|()| output.flush())
                .map_err(|err| error(err, 1))?,
            Ok(Err(err)) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Ok(Err(err)) => return Err(error(err, 1)),
            Err(_) => continue,
        }
        // A continuously writable container must not starve Start/Wait RPCs.
        tokio::task::yield_now().await;
    }
}

async fn local_attach(command: &Invocation, name: &str) -> Result<i32, CliError> {
    let id = rpc::inspect_id(&command.host, name)
        .await
        .map_err(|err| error(err, 255))?;
    local_start(&command.host, &id, None, false, false).await
}

pub(super) async fn local_start(
    host: &str,
    id: &str,
    input: Option<Input>,
    tty: bool,
    auto_remove: bool,
) -> Result<i32, CliError> {
    // FIFOs are shared with the local daemon, under a unique directory with mode 0700.
    let directory = tempfile::Builder::new()
        .prefix("risula-start-")
        .tempdir_in("/tmp")
        .map_err(|err| error(err, 1))?;
    let stdout = directory.path().join("out");
    let stderr = directory.path().join("err");
    let (out_reader, out_writer) = open_fifo(&stdout).map_err(|err| error(err, 1))?;
    let (err_reader, err_writer) = open_fifo(&stderr).map_err(|err| error(err, 1))?;
    let stdin = directory.path().join("in");
    let (in_reader, in_writer) = if input.is_some() {
        let (reader, writer) = open_fifo(&stdin).map_err(|err| error(err, 1))?;
        (
            Some(reader),
            Some(AsyncFd::new(writer).map_err(|err| error(err, 1))?),
        )
    } else {
        (None, None)
    };
    let attach_stdin = input.is_some();
    let start_and_wait = async {
        start_result(
            rpc::start_container(
                host,
                proto::StartRequest {
                    id: id.into(),
                    stdin: if attach_stdin {
                        stdin.to_string_lossy().into_owned()
                    } else {
                        String::new()
                    },
                    attach_stdin,
                    stdout: stdout.to_string_lossy().into_owned(),
                    stderr: stderr.to_string_lossy().into_owned(),
                    attach_stdout: true,
                    attach_stderr: true,
                },
            )
            .await,
            false,
        )?;
        drop((out_writer, err_writer, in_reader));
        rpc::wait_for(host, id, auto_remove)
            .await
            .map_err(|err| error(err, 125))
    };
    let output = async {
        tokio::try_join!(
            copy_output(out_reader, std::io::stdout()),
            copy_output(err_reader, std::io::stderr()),
        )
        .map(|_| ())
    };
    let input_copy = async {
        match (input, in_writer) {
            (Some(input), Some(writer)) => console::copy_input(input, writer, tty)
                .await
                .map_err(|err| error(err, 1)),
            _ => std::future::pending().await,
        }
    };
    let io = async {
        tokio::select! {
            result = output => result,
            result = input_copy => result,
        }
    };
    let (exit_code, ()) = tokio::try_join!(start_and_wait, io)?;
    Ok(exit_code as i32)
}

pub async fn run(command: &Invocation) -> i32 {
    let mut attach = false;
    for option in command
        .options
        .iter()
        .filter(|option| option.name == "attach")
    {
        match parse_bool(option.value.as_deref(), "attach") {
            Ok(value) => attach = value,
            Err(message) => {
                eprintln!("risula start: {message}");
                return 125;
            }
        }
    }
    if attach {
        let name = &command.args[0];
        let result = if command.host.starts_with("tcp://") {
            match start_result(rpc::remote_start(&command.host, name).await, true) {
                Ok(()) => rpc::wait_container(&command.host, name)
                    .await
                    .map(|code| code as i32)
                    .map_err(|err| error(err, 125)),
                Err(err) => Err(err),
            }
        } else {
            local_attach(command, name).await
        };
        return match result {
            Ok(code) => code,
            Err(err) => {
                eprintln!("risula start: {name}: {}", err.message);
                err.exit_code
            }
        };
    }

    let mut exit_code = 0;
    for name in &command.args {
        let result = if name.is_empty() {
            Err(error("container name cannot be empty", 1))
        } else {
            start_result(
                rpc::start_container(
                    &command.host,
                    proto::StartRequest {
                        id: name.into(),
                        ..Default::default()
                    },
                )
                .await,
                false,
            )
        };
        if let Err(err) = result {
            eprintln!("risula start: {name}: {}", err.message);
            exit_code = 1;
        }
    }
    exit_code
}
