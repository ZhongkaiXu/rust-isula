use super::connect_with_timeout;
use std::collections::HashMap;

pub mod proto {
    tonic::include_proto!("network");
}

pub async fn list(
    host: &str,
    filters: HashMap<String, String>,
) -> Result<Vec<proto::NetworkInfo>, String> {
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::network_service_client::NetworkServiceClient::new(channel)
        .list(proto::NetworkListRequest { filters })
        .await
        .map_err(|e| format!("network list RPC failed: {e}"))?
        .into_inner();
    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(response.networks)
}

pub async fn create(
    host: &str,
    request: proto::NetworkCreateRequest,
) -> Result<proto::NetworkCreateResponse, String> {
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::network_service_client::NetworkServiceClient::new(channel)
        .create(request)
        .await
        .map_err(|e| format!("network create RPC failed: {e}"))?
        .into_inner();
    if response.cc != 0 || response.name.is_empty() {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(response)
}

pub async fn inspect(host: &str, name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("Missing network name in the request".into());
    }
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::network_service_client::NetworkServiceClient::new(channel)
        .inspect(proto::NetworkInspectRequest { name: name.into() })
        .await
        .map_err(|e| format!("network inspect RPC failed: {e}"))?
        .into_inner();
    if response.cc != 0 || response.network_json.is_empty() {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(response.network_json)
}
