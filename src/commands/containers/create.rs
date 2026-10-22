mod config;
mod parse;

use crate::cli::{CliError, Invocation};
use crate::{commands::images::pull, grpc};

pub async fn run(command: &Invocation) -> Result<(), CliError> {
    let (request, policy) = config::build(command)?;
    let failure = |message| CliError {
        message,
        exit_code: 1,
    };
    if policy == "always" {
        pull::pull(&command.host, &request.image)
            .await
            .map_err(failure)?;
    }
    let mut response = grpc::create_container(&command.host, request.clone())
        .await
        .map_err(failure)?;
    if response.cc != 0 && policy == "missing" && response.errmsg.contains("No such image") {
        eprintln!("Unable to find image '{}' locally", request.image);
        pull::pull(&command.host, &request.image)
            .await
            .map_err(failure)?;
        response = grpc::create_container(&command.host, request)
            .await
            .map_err(failure)?;
    }
    if response.cc != 0 {
        return Err(failure(if response.errmsg.is_empty() {
            format!("create failed (code {})", response.cc)
        } else {
            response.errmsg
        }));
    }
    if response.id.is_empty() {
        return Err(failure("create RPC returned no container ID".into()));
    }
    println!("{}", response.id);
    Ok(())
}
