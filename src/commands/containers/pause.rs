use crate::cli::Invocation;
use crate::grpc;

pub async fn run(command: &Invocation) -> bool {
    let mut success = true;
    for name in &command.args {
        if name.is_empty() {
            eprintln!("risula pause: container name cannot be empty");
            success = false;
            continue;
        }
        match grpc::pause_container(&command.host, name).await {
            Ok(()) => println!("{name}"),
            Err(error) => {
                eprintln!("risula pause: {name}: {error}");
                success = false;
            }
        }
    }
    success
}
