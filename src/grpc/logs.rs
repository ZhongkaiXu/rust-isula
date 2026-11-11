use tonic::{Status, Streaming};

use super::{connect_with_timeout, proto};

pub async fn logs(
    host: &str,
    request: proto::LogsRequest,
) -> Result<Streaming<proto::LogsResponse>, Status> {
    let channel = connect_with_timeout(host, None)
        .await
        .map_err(Status::unavailable)?;
    proto::container_service_client::ContainerServiceClient::new(channel)
        .logs(request)
        .await
        .map(tonic::Response::into_inner)
}
