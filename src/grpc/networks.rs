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
