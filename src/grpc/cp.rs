use std::fs::File;
use std::io::{Read, Seek, Write};

use tonic::Request;

use super::{connect_with_timeout, proto};

pub async fn download(host: &str, id: &str, source: &str) -> Result<(File, bool), String> {
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::container_service_client::ContainerServiceClient::new(channel)
        .copy_from_container(proto::CopyFromContainerRequest {
            id: id.into(),
            srcpath: source.into(),
            runtime: String::new(),
        })
        .await
        .map_err(|err| err.to_string())?;
    let stat = response
        .metadata()
        .get("isulad-container-path-stat")
        .ok_or("missing container path stat")?
        .to_str()
        .map_err(|err| err.to_string())?;
    let stat: serde_json::Value =
        serde_json::from_str(stat).map_err(|err| format!("invalid path stat: {err}"))?;
    let mode = stat
        .get("Mode")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing path stat mode")?;
    let mut stream = response.into_inner();
    let mut file = tempfile::tempfile().map_err(|err| err.to_string())?;
    while let Some(frame) = stream.message().await.map_err(|err| err.to_string())? {
        file.write_all(&frame.data).map_err(|err| err.to_string())?;
    }
    file.rewind().map_err(|err| err.to_string())?;
    Ok((file, mode & 0o170000 == 0o040000))
}

pub async fn upload(
    host: &str,
    id: &str,
    source: &str,
    is_dir: bool,
    destination: &str,
    mut file: File,
) -> Result<(), String> {
    let channel = connect_with_timeout(host, None).await?;
    let metadata = serde_json::json!({ "id": id, "srcPath": source, "srcIsdir": is_dir, "dstPath": destination });
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
    let (sender, receiver) = tokio::sync::mpsc::channel(2);
    let mut request = Request::new(tokio_stream::wrappers::ReceiverStream::new(receiver));
    request.metadata_mut().insert(
        "isulad-copy-to-container",
        json.parse().map_err(|_| "invalid copy metadata")?,
    );
    let send = async move {
        let mut buffer = [0; 32 * 1024];
        loop {
            let size = file.read(&mut buffer).map_err(|err| err.to_string())?;
            if size == 0 {
                break;
            }
            if sender
                .send(proto::CopyToContainerRequest {
                    data: buffer[..size].to_vec(),
                })
                .await
                .is_err()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        Ok::<_, String>(())
    };
    let receive = async {
        let mut stream = proto::container_service_client::ContainerServiceClient::new(channel)
            .copy_to_container(request)
            .await
            .map_err(|err| err.to_string())?
            .into_inner();
        // The final gRPC status can report an error after a finish frame.
        while stream
            .message()
            .await
            .map_err(|err| err.to_string())?
            .is_some()
        {}
        Ok::<_, String>(())
    };
    tokio::try_join!(send, receive)?;
    Ok(())
}
