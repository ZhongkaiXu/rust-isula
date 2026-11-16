use super::{connect_with_timeout, proto};

pub async fn inspect(host: &str, id: &str) -> Result<String, String> {
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::container_service_client::ContainerServiceClient::new(channel)
        .inspect(proto::InspectContainerRequest {
            id: id.into(),
            bformat: false,
            timeout: 120,
        })
        .await
        .map_err(|error| format!("inspect RPC failed: {error}"))?
        .into_inner();
    if response.cc != 0 {
        return Err(response.errmsg);
    }
    Ok(response.container_json)
}
