use super::connect_with_timeout;

pub mod proto {
    tonic::include_proto!("volume");
}

pub async fn list(host: &str) -> Result<Vec<proto::Volume>, String> {
    let channel = connect_with_timeout(host, None).await?;
    let response = proto::volume_service_client::VolumeServiceClient::new(channel)
        .list(proto::ListVolumeRequest {})
        .await
        .map_err(|e| format!("volume list RPC failed: {e}"))?
        .into_inner();
    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(response.volumes)
}
