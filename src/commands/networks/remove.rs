use crate::cli::Invocation;
use crate::grpc::networks;

pub async fn run(command: &Invocation) -> Result<(), String> {
    for name in &command.args {
        let warning = networks::remove(&command.host, name)
            .await
            .map_err(|error| format!("{error}\nRemove network {name} failed"))?;
        if !warning.is_empty() {
            eprintln!("{warning}");
        }
        println!("{name}");
    }
    Ok(())
}
