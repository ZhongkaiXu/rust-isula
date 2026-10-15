use super::parse_time;
use crate::cli::Invocation;
use crate::grpc;

// Err is an invalid option; Ok(false) means at least one container failed.
pub async fn run(command: &Invocation) -> Result<bool, String> {
    let mut timeout = 10;
    for option in &command.options {
        if option.name == "time" {
            timeout = parse_time(option.value.as_deref().unwrap_or(""))?;
        }
    }

    let mut success = true;
    for name in &command.args {
        if name.is_empty() {
            eprintln!("risula restart: container name cannot be empty");
            success = false;
            continue;
        }
        match grpc::restart_container(&command.host, name, timeout).await {
            Ok(()) => println!("{name}"),
            Err(error) => {
                eprintln!("risula restart: {name}: {error}");
                success = false;
            }
        }
    }
    Ok(success)
}
