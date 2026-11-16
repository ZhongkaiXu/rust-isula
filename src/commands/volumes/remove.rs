use crate::cli::Invocation;
use crate::grpc::volumes;

pub async fn run(command: &Invocation) -> bool {
    let mut success = true;
    for name in &command.args {
        let result = if name.is_empty() {
            Err("Missing volume name in the request".into())
        } else {
            volumes::remove(&command.host, name).await
        };
        match result {
            Ok(()) => println!("{name}"),
            Err(error) => {
                eprintln!("risula volume rm: {name}: {error}");
                success = false;
            }
        }
    }
    success
}
