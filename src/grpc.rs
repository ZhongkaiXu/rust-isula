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
    connect_with_timeout(host, Some(Duration::from_secs(5))).await
}

async fn connect_with_timeout(
    host: &str,
    request_timeout: Option<Duration>,
) -> Result<Channel, String> {
    let address = match host.strip_prefix("tcp://") {
        Some(address) => format!("http://{address}"),
        None => host.to_string(),
    };
    let endpoint = Endpoint::from_shared(address)
        .map_err(|error| format!("invalid daemon address: {error}"))?
        .connect_timeout(Duration::from_secs(5));
    let endpoint = match request_timeout {
        Some(duration) => endpoint.timeout(duration),
        None => endpoint,
    };
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

pub async fn login(host: &str, server: &str, username: &str, password: &str) -> Result<(), String> {
    if !host.starts_with("unix://") {
        return Err("login requires a Unix socket; TCP credentials are not encrypted".to_string());
    }
    let channel = connect_with_timeout(host, None).await?;
    let mut client = images_proto::images_service_client::ImagesServiceClient::new(channel);
    let response = client
        .login(images_proto::LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
            server: server.to_string(),
            r#type: "oci".to_string(),
        })
        .await
        .map_err(|error| format!("login RPC failed: {error}"))?
        .into_inner();

    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(())
}

pub async fn tag_image(host: &str, source: &str, destination: &str) -> Result<(), String> {
    let channel = connect(host).await?;
    let mut client = images_proto::images_service_client::ImagesServiceClient::new(channel);
    let response = client
        .tag(images_proto::TagImageRequest {
            src_name: source.to_string(),
            dest_name: destination.to_string(),
        })
        .await
        .map_err(|error| format!("tag RPC failed: {error}"))?
        .into_inner();

    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(())
}

pub async fn load_image(host: &str, file: &str, tag: &str) -> Result<(), String> {
    let channel = connect(host).await?;
    let mut client = images_proto::images_service_client::ImagesServiceClient::new(channel);
    let response = client
        .load(images_proto::LoadImageRequest {
            file: file.to_string(),
            r#type: "oci".to_string(),
            tag: tag.to_string(),
        })
        .await
        .map_err(|error| format!("load RPC failed: {error}"))?
        .into_inner();

    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(())
}

pub async fn import_image(host: &str, file: &str, tag: &str) -> Result<String, String> {
    let channel = connect_with_timeout(host, None).await?;
    let mut client = images_proto::images_service_client::ImagesServiceClient::new(channel);
    let response = client
        .import(images_proto::ImportRequest {
            file: file.to_string(),
            tag: tag.to_string(),
        })
        .await
        .map_err(|error| format!("import RPC failed: {error}"))?
        .into_inner();

    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(response.id)
}

pub async fn pull_image(
    host: &str,
    name: &str,
    show_progress: bool,
) -> Result<tonic::Streaming<images_proto::PullImageResponse>, String> {
    let channel = connect_with_timeout(host, None).await?;
    let mut client = images_proto::images_service_client::ImagesServiceClient::new(channel);
    client
        .pull_image(images_proto::PullImageRequest {
            image: Some(images_proto::ImageSpec {
                image: name.to_string(),
                annotations: HashMap::new(),
            }),
            auth: None,
            is_progress_visible: show_progress,
        })
        .await
        .map(|response| response.into_inner())
        .map_err(|error| format!("pull RPC failed: {error}"))
}

pub async fn delete_image(host: &str, name: &str, force: bool) -> Result<(), String> {
    let channel = connect_with_timeout(host, None).await?;
    let mut client = images_proto::images_service_client::ImagesServiceClient::new(channel);
    let response = client
        .delete(images_proto::DeleteImageRequest {
            name: name.to_string(),
            force,
        })
        .await
        .map_err(|error| format!("delete RPC failed: {error}"))?
        .into_inner();

    if response.cc != 0 {
        return Err(format!(
            "server error (code {}): {}",
            response.cc, response.errmsg
        ));
    }
    Ok(())
}
