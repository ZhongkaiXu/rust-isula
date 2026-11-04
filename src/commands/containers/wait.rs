use crate::cli::Invocation;
use crate::grpc::start::wait_container;

pub async fn run(command: &Invocation) -> bool {
    let mut success = true;
    for name in &command.args {
        match wait_container(&command.host, name).await {
            Ok(exit_code) => println!("{exit_code}"),
            Err(error) => {
                eprintln!("risula wait: {name}: {error}");
                success = false;
            }
        }
    }
    success
}
