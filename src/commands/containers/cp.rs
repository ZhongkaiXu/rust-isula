mod archive;

use crate::{cli::Invocation, grpc::cp as rpc};

fn split(value: &str) -> (Option<&str>, &str) {
    match value.split_once(':') {
        Some(("", path)) => (None, path),
        Some((container, path)) => (Some(container), path),
        None => (None, value),
    }
}

pub async fn run(command: &Invocation) -> Result<(), String> {
    if command.args.len() != 2 {
        return Err("cp requires exactly two paths".into());
    }
    let (src_container, source) = split(&command.args[0]);
    let (dst_container, destination) = split(&command.args[1]);
    match (src_container, dst_container) {
        (Some(_), Some(_)) => Err("copying between containers is not supported".into()),
        (None, None) => Err("must specify at least one container source".into()),
        (None, Some(id)) => {
            let (file, path, is_dir) = archive::pack(source).map_err(|err| err.to_string())?;
            rpc::upload(&command.host, id, &path, is_dir, destination, file).await
        }
        (Some(id), None) => {
            let (file, is_dir) = rpc::download(&command.host, id, source).await?;
            archive::unpack(file, source, is_dir, destination).map_err(|err| err.to_string())
        }
    }
}
