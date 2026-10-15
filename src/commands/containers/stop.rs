use super::parse_time;
use crate::cli::Invocation;
use crate::grpc;

// Err is an invalid option; Ok(false) means at least one container failed.
pub async fn run(command: &Invocation) -> Result<bool, String> {
    let mut timeout = 10;
    let mut force = false;
    for option in &command.options {
        match option.name {
            "time" => timeout = parse_time(option.value.as_deref().unwrap_or(""))?,
            "force" => {
                force = crate::cli::parse_bool(option.value.as_deref(), "force")?;
            }
            _ => {}
        }
    }
    if force {
        timeout = 0;
    }

    let mut success = true;
    for name in &command.args {
        if name.is_empty() {
            eprintln!("risula stop: container name cannot be empty");
            success = false;
            continue;
        }
        match grpc::stop_container(&command.host, name, force, timeout).await {
            Ok(response) => {
                if !response.errmsg.is_empty() {
                    eprintln!("{}", response.errmsg);
                }
                println!("{name}");
            }
            Err(error) => {
                eprintln!("risula stop: {name}: {error}");
                success = false;
            }
        }
    }
    Ok(success)
}
