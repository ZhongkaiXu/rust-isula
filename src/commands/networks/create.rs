use crate::cli::{parse_bool, Invocation};
use crate::grpc::networks::{self, proto::NetworkCreateRequest};

pub async fn run(command: &Invocation) -> Result<(), String> {
    let name = command.args.first().cloned().unwrap_or_default();
    if name.len() > 200 {
        return Err(format!("Network name '{name}' too long, max length:200"));
    }
    let mut request = NetworkCreateRequest {
        name,
        driver: "bridge".into(),
        ..Default::default()
    };
    for option in &command.options {
        let value = option.value.as_deref().unwrap_or_default();
        match option.name {
            "driver" => request.driver = value.into(),
            "gateway" => request.gateway = value.into(),
            "subnet" => request.subnet = value.into(),
            "internal" => request.internal = parse_bool(option.value.as_deref(), "internal")?,
            _ => {}
        }
    }
    let response = networks::create(&command.host, request).await?;
    if !response.errmsg.is_empty() {
        eprintln!("{}", response.errmsg);
    }
    println!("{}", response.name);
    Ok(())
}
