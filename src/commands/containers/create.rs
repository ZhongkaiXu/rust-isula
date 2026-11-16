mod config;
pub(super) mod parse;

use crate::cli::{CliError, Invocation};
use crate::{commands::images::pull, grpc};

pub(super) use config::build;

pub async fn run(command: &Invocation) -> Result<(), CliError> {
    let (request, policy) = build(command)?;
    println!("{}", create(&command.host, request, &policy).await?);
    Ok(())
}

pub(super) async fn create(
    host: &str,
    request: grpc::proto::CreateRequest,
    policy: &str,
) -> Result<String, CliError> {
    let failure = |message| CliError {
        message,
        exit_code: 1,
    };
    if policy == "always" {
        pull::pull(host, &request.image).await.map_err(failure)?;
    }
    let mut response = grpc::create_container(host, request.clone())
        .await
        .map_err(failure)?;
    if response.cc != 0 && policy == "missing" && response.errmsg.contains("No such image") {
        eprintln!("Unable to find image '{}' locally", request.image);
        pull::pull(host, &request.image).await.map_err(failure)?;
        response = grpc::create_container(host, request)
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
    Ok(response.id)
}
