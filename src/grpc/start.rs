use std::io::Write;

use tonic::{Request, Status};

use super::{connect_with_timeout, proto};

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
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::container_service_client::ContainerServiceClient::new(channel)
        .wait(proto::WaitRequest {
            id: id.into(),
            condition: 0, // WAIT_CONDITION_STOPPED
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
    let channel = connect_with_timeout(host, None)
        .await
        .map_err(Status::unavailable)?;
    // start -a attaches output only. Keep the request stream open until finish.
    let (sender, receiver) = tokio::sync::mpsc::channel::<proto::RemoteStartRequest>(1);
    let mut request = Request::new(tokio_stream::wrappers::ReceiverStream::new(receiver));
    request.metadata_mut().insert(
        "container-id",
        name.parse()
            .map_err(|_| Status::invalid_argument("invalid container name in metadata"))?,
    );
    for (key, value) in [
        ("attach-stdin", "false"),
        ("attach-stdout", "true"),
        ("attach-stderr", "true"),
    ] {
        request.metadata_mut().insert(key, value.parse().unwrap());
    }
    let mut stream = proto::container_service_client::ContainerServiceClient::new(channel)
        .remote_start(request)
        .await?
        .into_inner();
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
                .map_err(|error| Status::internal(format!("write output: {error}")))?;
        }
    }
    drop(sender);
    let trailers = stream.trailers().await?.unwrap_or_default();
    let cc = match trailers.get("cc") {
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
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .into(),
        ..Default::default()
    })
}
