use std::io::Read;

use tokio::io::unix::AsyncFd;

use super::{console, start};
use crate::cli::{parse_bool, CliError, Invocation};
use crate::grpc::{exec as rpc, proto};

fn error(message: impl ToString) -> CliError {
    CliError {
        message: message.to_string(),
        exit_code: 1,
    }
}

pub fn build(command: &Invocation) -> Result<proto::ExecRequest, CliError> {
    if command.args.len() < 2 || command.args[0].is_empty() {
        return Err(error("exec requires a container and command"));
    }
    let mut request = proto::ExecRequest {
        container_id: command.args[0].clone(),
        argv: command.args[1..].to_vec(),
        attach_stdout: true,
        attach_stderr: true,
        ..Default::default()
    };
    let mut detach = false;
    for option in &command.options {
        let value = option.value.as_deref().unwrap_or_default();
        match option.name {
            "detach" => detach = parse_bool(option.value.as_deref(), "detach")?,
            "interactive" => {
                request.open_stdin = parse_bool(option.value.as_deref(), "interactive")?
            }
            "tty" => request.tty = parse_bool(option.value.as_deref(), "tty")?,
            "user" => request.user = value.into(),
            "workdir" => {
                if !value.starts_with('/') {
                    return Err(error("workdir must be an absolute path"));
                }
                request.workdir = value.into();
            }
            "env" => {
                let key = value.split('=').next().unwrap_or_default();
                if key.is_empty() {
                    return Err(error("environment variable name cannot be empty"));
                }
                if value.contains('=') {
                    request.env.push(value.into());
                } else if let Ok(value) = std::env::var(key) {
                    request.env.push(format!("{key}={value}"));
                }
            }
            _ => {}
        }
    }
    request.attach_stdout = !detach;
    request.attach_stderr = !detach;
    request.open_stdin &= !detach;
    request.attach_stdin = request.open_stdin;
    Ok(request)
}

fn result(response: proto::ExecResponse) -> Result<i32, CliError> {
    if response.cc != 0 {
        return Err(CliError {
            message: response.errmsg,
            exit_code: if response.exit_code != 0 {
                response.exit_code as i32
            } else {
                1
            },
        });
    }
    Ok(response.exit_code as i32)
}

async fn local(
    host: &str,
    mut request: proto::ExecRequest,
    input: Option<console::Input>,
) -> Result<i32, CliError> {
    if !request.attach_stdout {
        return result(rpc::exec(host, request).await.map_err(error)?);
    }
    let directory = tempfile::Builder::new()
        .prefix("risula-exec-")
        .tempdir_in("/tmp")
        .map_err(error)?;
    let stdout = directory.path().join("out");
    let stderr = directory.path().join("err");
    let (out_reader, out_writer) = start::open_fifo(&stdout).map_err(error)?;
    let (err_reader, err_writer) = start::open_fifo(&stderr).map_err(error)?;
    request.stdout = stdout.to_string_lossy().into_owned();
    request.stderr = stderr.to_string_lossy().into_owned();
    let (in_reader, in_writer) = if input.is_some() {
        let stdin = directory.path().join("in");
        let (reader, writer) = start::open_fifo(&stdin).map_err(error)?;
        request.stdin = stdin.to_string_lossy().into_owned();
        (Some(reader), Some(AsyncFd::new(writer).map_err(error)?))
    } else {
        (None, None)
    };
    let tty = request.tty && unsafe { libc::isatty(0) } == 1;
    let execute = async {
        let response = rpc::exec(host, request).await.map_err(error)?;
        // Exec responds when the process exits. Release our extra writers to drain EOF.
        drop((out_writer, err_writer, in_reader));
        result(response)
    };
    let output = async {
        tokio::try_join!(
            start::copy_output(out_reader, std::io::stdout()),
            start::copy_output(err_reader, std::io::stderr())
        )?;
        Ok::<_, CliError>(())
    };
    let input_copy = async {
        if let (Some(input), Some(writer)) = (input, in_writer) {
            console::copy_input(input, writer, tty)
                .await
                .map_err(error)?;
        }
        std::future::pending::<Result<(), CliError>>().await
    };
    let io = async {
        tokio::select! { result = output => result, result = input_copy => result }
    };
    let (code, ()) = tokio::try_join!(execute, io)?;
    Ok(code)
}

pub async fn run(command: &Invocation) -> Result<i32, CliError> {
    let mut request = build(command)?;
    request.container_id = crate::grpc::start::inspect_id(&command.host, &request.container_id)
        .await
        .map_err(error)?;
    let mut random = [0; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut random))
        .map_err(error)?;
    request.suffix = random.iter().map(|byte| format!("{byte:02x}")).collect();
    let id = request.container_id.clone();
    let suffix = request.suffix.clone();
    let tty = request.tty && request.attach_stdout;
    let _terminal = console::Terminal::new(tty && request.attach_stdin).map_err(error)?;
    let input = if request.attach_stdin {
        Some(console::Input::new().map_err(error)?)
    } else {
        None
    };
    let execute = async {
        if command.host.starts_with("tcp://") {
            result(
                rpc::remote_exec(&command.host, request, input)
                    .await
                    .map_err(error)?,
            )
        } else {
            local(&command.host, request, input).await
        }
    };
    tokio::select! {
        result = execute => result,
        _ = console::resize_exec(&command.host, &id, &suffix, tty) => unreachable!(),
    }
}
