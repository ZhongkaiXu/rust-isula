use std::io::Write;

use tonic::{Request, Status};

use super::{connect_with_timeout, proto};
use crate::commands::containers::console::Input;

pub async fn exec(host: &str, request: proto::ExecRequest) -> Result<proto::ExecResponse, Status> {
    let channel = connect_with_timeout(host, None)
        .await
        .map_err(Status::unavailable)?;
    proto::container_service_client::ContainerServiceClient::new(channel)
        .exec(request)
        .await
        .map(|response| response.into_inner())
}

pub(crate) async fn remote_exec(
    host: &str,
    exec: proto::ExecRequest,
    input: Option<Input>,
) -> Result<proto::ExecResponse, Status> {
    let channel = connect_with_timeout(host, None)
        .await
        .map_err(Status::unavailable)?;
    let mut metadata = serde_json::json!({
        "container_id": exec.container_id, "tty": exec.tty,
        "open_stdin": exec.open_stdin, "attach_stdin": exec.attach_stdin,
        "attach_stdout": exec.attach_stdout, "attach_stderr": exec.attach_stderr,
        "argv": exec.argv, "env": exec.env, "suffix": exec.suffix,
    });
    if !exec.user.is_empty() {
        metadata["user"] = exec.user.into();
    }
    if !exec.workdir.is_empty() {
        metadata["workdir"] = exec.workdir.into();
    }
    // gRPC ASCII metadata needs escaped JSON for non-ASCII arguments.
    let json: String = metadata
        .to_string()
        .encode_utf16()
        .map(|unit| {
            if unit < 128 {
                char::from_u32(unit.into()).unwrap().to_string()
            } else {
                format!("\\u{unit:04x}")
            }
        })
        .collect();
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    let mut request = Request::new(tokio_stream::wrappers::ReceiverStream::new(receiver));
    request.metadata_mut().insert(
        "isulad-remote-exec",
        json.parse()
            .map_err(|_| Status::invalid_argument("invalid exec metadata"))?,
    );
    let send_input = async {
        if let Some(mut input) = input {
            let mut buffer = [0; 16 * 1024];
            loop {
                let size = input
                    .read(&mut buffer)
                    .await
                    .map_err(|err| Status::internal(format!("read stdin: {err}")))?;
                if sender
                    .send(proto::RemoteExecRequest {
                        cmd: if size == 0 {
                            vec![]
                        } else {
                            vec![buffer[..size].to_vec()]
                        },
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
    let receive = async {
        let response = proto::container_service_client::ContainerServiceClient::new(channel)
            .remote_exec(request)
            .await?;
        let headers = response.metadata().clone();
        let mut stream = response.into_inner();
        while let Some(response) = stream.message().await? {
            for (data, output) in [
                (&response.stdout, &mut std::io::stdout() as &mut dyn Write),
                (&response.stderr, &mut std::io::stderr() as &mut dyn Write),
            ] {
                output
                    .write_all(data)
                    .and_then(|()| output.flush())
                    .map_err(|err| Status::internal(format!("write output: {err}")))?;
            }
            if response.finish {
                break;
            }
        }
        Ok::<_, Status>((stream, headers))
    };
    let (mut stream, headers) = tokio::select! {
        result = receive => result?,
        result = send_input => { result?; unreachable!() },
    };
    // The server joins its stdin reader before sending trailers.
    drop(sender);
    let trailers = stream.trailers().await?.unwrap_or_default();
    let number = |key| -> Result<u32, Status> {
        match trailers.get(key).or_else(|| headers.get(key)) {
            Some(value) => value
                .to_str()
                .ok()
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| Status::internal(format!("invalid exec {key}"))),
            None => Ok(0),
        }
    };
    Ok(proto::ExecResponse {
        cc: number("cc")?,
        exit_code: number("exit_code")?,
        errmsg: trailers
            .get("errmsg")
            .or_else(|| headers.get("errmsg"))
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .into(),
        ..Default::default()
    })
}
