use tonic::{Status, Streaming};

use super::{connect_with_timeout, proto};

pub async fn events(
    host: &str,
    request: proto::EventsRequest,
) -> Result<Streaming<proto::Event>, Status> {
    let channel = connect_with_timeout(host, None)
        .await
        .map_err(Status::unavailable)?;
    proto::container_service_client::ContainerServiceClient::new(channel)
        .events(request)
        .await
        .map(tonic::Response::into_inner)
}
