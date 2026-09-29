use std::collections::HashMap;
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};

pub mod proto {
    tonic::include_proto!("containers");
}

pub mod images_proto {
    tonic::include_proto!("images");
}

pub async fn connect(host: &str) -> Result<Channel, String> {
    let address = match host.strip_prefix("tcp://") {
        Some(address) => format!("http://{address}"),
        None => host.to_string(),
    };
    let endpoint = Endpoint::from_shared(address)
        .map_err(|error| format!("invalid daemon address: {error}"))?
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(5));
    endpoint
        .connect()
        .await
        .map_err(|error| format!("cannot connect to {host}: {error}"))
}

pub async fn version(host: &str) -> Result<proto::VersionResponse, String> {
    let channel = connect(host).await?;
    let mut client = proto::container_service_client::ContainerServiceClient::new(channel);
    let response = client
        .version(proto::VersionRequest {})
        .await
        .map_err(|error| format!("version RPC failed: {error}"))?
        .into_inner();

    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(response)
}

pub async fn info(host: &str) -> Result<proto::InfoResponse, String> {
    let channel = connect(host).await?;
    let mut client = proto::container_service_client::ContainerServiceClient::new(channel);
    let response = client
        .info(proto::InfoRequest {})
        .await
        .map_err(|error| format!("info RPC failed: {error}"))?
        .into_inner();

    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(response)
}

pub async fn list_images(
    host: &str,
    filters: HashMap<String, String>,
) -> Result<Vec<images_proto::Image>, String> {
    let channel = connect(host).await?;
    let mut client = images_proto::images_service_client::ImagesServiceClient::new(channel);
    let response = client
        .list(images_proto::ListImagesRequest { filters })
        .await
        .map_err(|error| format!("images RPC failed: {error}"))?
        .into_inner();

    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(response.images)
}
