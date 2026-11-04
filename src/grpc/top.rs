use super::{connect_with_timeout, proto};

pub async fn processes(
    host: &str,
    id: &str,
    args: Vec<String>,
) -> Result<proto::TopResponse, String> {
    let channel = connect_with_timeout(host, None).await?;
    let mut client = proto::container_service_client::ContainerServiceClient::new(channel);
    let inspect = client
        .inspect(proto::InspectContainerRequest {
            id: id.into(),
            bformat: false,
            timeout: 120,
        })
        .await
        .map_err(|error| format!("inspect RPC failed: {error}"))?
        .into_inner();
    if inspect.cc != 0 {
        return Err(inspect.errmsg);
    }
    let data: serde_json::Value = serde_json::from_str(&inspect.container_json)
        .map_err(|error| format!("invalid inspect response: {error}"))?;
    if let Some(state) = data.get("State").filter(|state| !state.is_null()) {
        if state.get("Running").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err("container is stopped; start it first".into());
        }
        if state.get("Restarting").and_then(serde_json::Value::as_bool) == Some(true) {
            return Err("container is restarting; wait until it is running".into());
        }
    }
    client
        .top(proto::TopRequest {
            id: id.into(),
            args,
        })
        .await
        .map(tonic::Response::into_inner)
        .map_err(|error| format!("top RPC failed: {error}"))
}
