use crate::cli::{CliError, Invocation};
use crate::grpc::{self, proto};

fn error(message: impl ToString, exit_code: i32) -> CliError {
    CliError {
        message: message.to_string(),
        exit_code,
    }
}

pub fn build(command: &Invocation) -> Result<proto::ExportRequest, CliError> {
    if command.args.len() != 1 {
        return Err(error("Export requires exactly 1 container name", 125));
    }
    let output = command
        .options
        .iter()
        .rev()
        .find(|o| o.name == "output")
        .and_then(|o| o.value.as_deref())
        .ok_or_else(|| error("Missing output file, use -o,--output option", 125))?;
    // The daemon writes the archive. Do not open or canonicalize this path locally.
    let file = if output.starts_with('/') {
        output.to_owned()
    } else {
        let cwd = std::env::current_dir().map_err(|e| error(e, 1))?;
        let cwd = cwd
            .to_str()
            .ok_or_else(|| error("current directory is not valid UTF-8", 1))?;
        let file = format!("{cwd}/{output}");
        if file.len() >= libc::PATH_MAX as usize {
            return Err(error("filename too long", 125));
        }
        file
    };
    if command.args[0].is_empty() {
        return Err(error("Missing container name in the request", 1));
    }
    Ok(proto::ExportRequest {
        id: command.args[0].clone(),
        file,
    })
}

pub async fn run(command: &Invocation) -> Result<(), CliError> {
    grpc::export_container(&command.host, build(command)?)
        .await
        .map_err(|e| error(e, 1))
}
