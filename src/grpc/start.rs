use std::io::Write;

use tonic::{Request, Status};

use super::{connect_with_timeout, proto};
use crate::commands::containers::console::Input;

pub async fn start_container(
    host: &str,
    request: proto::StartRequest,
) -> Result<proto::StartResponse, Status> {
    let channel = connect_with_timeout(host, None)
        .await
        .map_err(Status::unavailable)?;
    proto::container_service_client::ContainerServiceClient::new(channel)
        .start(request)
        .await
        .map(|response| response.into_inner())
}

pub async fn inspect_id(host: &str, name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("container name cannot be empty".into());
    }
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::container_service_client::ContainerServiceClient::new(channel)
        .inspect(proto::InspectContainerRequest {
            id: name.into(),
            bformat: false,
            timeout: 120,
        })
        .await
        .map_err(|error| format!("inspect RPC failed: {error}"))?
        .into_inner();
    if response.cc != 0 {
        return Err(response.errmsg);
    }
    let json: serde_json::Value = serde_json::from_str(&response.container_json)
        .map_err(|error| format!("invalid inspect response: {error}"))?;
    json.get("Id")
        .and_then(|id| id.as_str())
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "inspect response has no container ID".into())
}

pub async fn wait_container(host: &str, id: &str) -> Result<u32, String> {
    wait_for(host, id, false).await
}

pub(crate) async fn wait_for(host: &str, id: &str, auto_remove: bool) -> Result<u32, String> {
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::container_service_client::ContainerServiceClient::new(channel)
        .wait(proto::WaitRequest {
            id: id.into(),
            condition: u32::from(auto_remove), // STOPPED = 0, REMOVED = 1
        })
        .await
        .map_err(|error| format!("wait RPC failed: {error}"))?
        .into_inner();
    if response.cc != 0 {
        return Err(response.errmsg);
    }
    Ok(response.exit_code)
}

pub async fn remote_start(host: &str, name: &str) -> Result<proto::StartResponse, Status> {
    remote_start_with_input(host, name, None, true).await
}

pub(crate) async fn remote_start_with_input(
    host: &str,
    name: &str,
    input: Option<Input>,
    attach_output: bool,
) -> Result<proto::StartResponse, Status> {
    let channel = connect_with_timeout(host, None)
        .await
        .map_err(Status::unavailable)?;
    // Keep the request stream open until output finishes, even after stdin EOF.
    let (sender, receiver) = tokio::sync::mpsc::channel::<proto::RemoteStartRequest>(1);
    let mut request = Request::new(tokio_stream::wrappers::ReceiverStream::new(receiver));
    request.metadata_mut().insert(
        "container-id",
        name.parse()
            .map_err(|_| Status::invalid_argument("invalid container name in metadata"))?,
    );
    for (key, value) in [
        (
            "attach-stdin",
            if input.is_some() { "true" } else { "false" },
        ),
        (
            "attach-stdout",
            if attach_output { "true" } else { "false" },
        ),
        (
            "attach-stderr",
            if attach_output { "true" } else { "false" },
        ),
    ] {
        request.metadata_mut().insert(key, value.parse().unwrap());
    }
    let send_input = async {
        if let Some(mut input) = input {
            let mut buffer = [0; 16 * 1024];
            loop {
                let size = input
                    .read(&mut buffer)
                    .await
                    .map_err(|err| Status::internal(format!("read stdin: {err}")))?;
                if sender
                    .send(proto::RemoteStartRequest {
                        stdin: buffer[..size].to_vec(),
                        finish: size == 0,
                    })
                    .await
                    .is_err()
                    || size == 0
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        }
        std::future::pending::<Result<(), Status>>().await
    };
    let receive_output = async {
        let response = proto::container_service_client::ContainerServiceClient::new(channel)
            .remote_start(request)
            .await?;
        // A response without data can carry its final status in the initial headers.
        let headers = response.metadata().clone();
        let mut stream = response.into_inner();
        while let Some(response) = stream.message().await? {
            if response.finish {
                break;
            }
            for (data, output) in [
                (&response.stdout, &mut std::io::stdout() as &mut dyn Write),
                (&response.stderr, &mut std::io::stderr() as &mut dyn Write),
            ] {
                output
                    .write_all(data)
                    .and_then(|()| output.flush())
                    .map_err(|err| Status::internal(format!("write output: {err}")))?;
            }
        }
        Ok::<_, Status>((stream, headers))
    };
    let (mut stream, headers) = tokio::select! {
        result = receive_output => result?,
        result = send_input => { result?; unreachable!() },
    };
    drop(sender);
    let trailers = stream.trailers().await?.unwrap_or_default();
    let cc = match trailers.get("cc").or_else(|| headers.get("cc")) {
        Some(value) => value
            .to_str()
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| Status::internal("invalid remote start status"))?,
        None => 0,
    };
    Ok(proto::StartResponse {
        cc,
        errmsg: trailers
            .get("errmsg")
            .or_else(|| headers.get("errmsg"))
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .into(),
        ..Default::default()
    })
}

pub(crate) async fn resize_exec(
    host: &str,
    id: &str,
    suffix: &str,
    height: u32,
    width: u32,
) -> Result<(), String> {
    let channel = super::connect(host).await?;
    let response = proto::container_service_client::ContainerServiceClient::new(channel)
        .resize(proto::ResizeRequest {
            id: id.into(),
            suffix: suffix.into(),
            height,
            width,
        })
        .await
        .map_err(|err| format!("resize RPC failed: {err}"))?
        .into_inner();
    if response.cc != 0 {
        return Err(response.errmsg);
    }
    Ok(())
}
