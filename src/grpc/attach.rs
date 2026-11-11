use std::io::Write;

use tonic::{Request, Status};

use super::{connect_with_timeout, proto};
use crate::commands::containers::console::Input;

pub async fn inspect(host: &str, name: &str) -> Result<(String, bool), String> {
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::container_service_client::ContainerServiceClient::new(channel)
        .inspect(proto::InspectContainerRequest {
            id: name.into(),
            bformat: false,
            timeout: 120,
        })
        .await
        .map_err(|err| format!("inspect RPC failed: {err}"))?
        .into_inner();
    if response.cc != 0 {
        return Err(response.errmsg);
    }
    let data: serde_json::Value = serde_json::from_str(&response.container_json)
        .map_err(|err| format!("invalid inspect response: {err}"))?;
    if let Some(state) = data.get("State").filter(|state| !state.is_null()) {
        if state.get("Running").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err("You cannot attach to a stopped container, start it first".into());
        }
        if state.get("Paused").and_then(serde_json::Value::as_bool) == Some(true) {
            return Err("You cannot attach to a paused container, unpause it first".into());
        }
        if state.get("Restarting").and_then(serde_json::Value::as_bool) == Some(true) {
            return Err(
                "You cannot attach to a restarting container, wait until it is running".into(),
            );
        }
    }
    let id = data
        .get("Id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or("inspect response has no container ID")?
        .to_owned();
    let tty = data
        .get("Config")
        .and_then(|config| config.get("Tty"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    Ok((id, tty))
}

pub(crate) async fn attach(host: &str, id: &str, mut input: Input) -> Result<(), Status> {
    let channel = connect_with_timeout(host, None)
        .await
        .map_err(Status::unavailable)?;
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    let mut request = Request::new(tokio_stream::wrappers::ReceiverStream::new(receiver));
    request.metadata_mut().insert(
        "container-id",
        id.parse()
            .map_err(|_| Status::invalid_argument("invalid container ID in metadata"))?,
    );
    for key in ["attach-stdin", "attach-stdout", "attach-stderr"] {
        request.metadata_mut().insert(key, "true".parse().unwrap());
    }
    let send_input = async {
        let mut buffer = [0; 16 * 1024];
        loop {
            let size = input
                .read(&mut buffer)
                .await
                .map_err(|err| Status::internal(format!("read stdin: {err}")))?;
            if sender
                .send(proto::AttachRequest {
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
        // stdin EOF must not discard output still arriving from the daemon.
        std::future::pending::<Result<(), Status>>().await
    };
    let receive = async {
        let response = proto::container_service_client::ContainerServiceClient::new(channel)
            .attach(request)
            .await?;
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
        result = receive => result?,
        result = send_input => { result?; unreachable!() },
    };
    // Release the server's stdin reader before waiting for final metadata.
    drop(sender);
    let trailers = stream.trailers().await?.unwrap_or_default();
    let cc = match trailers.get("cc").or_else(|| headers.get("cc")) {
        Some(value) => value
            .to_str()
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| Status::internal("invalid attach status"))?,
        None => 0,
    };
    if cc != 0 {
        let message = trailers
            .get("errmsg")
            .or_else(|| headers.get("errmsg"))
            .and_then(|value| value.to_str().ok())
            .unwrap_or("attach failed");
        return Err(Status::unknown(message));
    }
    Ok(())
}
