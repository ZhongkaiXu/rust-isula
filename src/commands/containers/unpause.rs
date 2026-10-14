use crate::cli::Invocation;
use crate::grpc;

pub async fn run(command: &Invocation) -> bool {
    let mut success = true;
    for name in &command.args {
        if name.is_empty() {
            eprintln!("risula unpause: container name cannot be empty");
            success = false;
            continue;
        }
        match grpc::unpause_container(&command.host, name).await {
            Ok(()) => println!("{name}"),
            Err(error) => {
                eprintln!("risula unpause: {name}: {error}");
                success = false;
            }
        }
    }
    success
}
