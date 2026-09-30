use crate::cli::Invocation;
use crate::grpc;

pub async fn run(command: &Invocation) -> Result<(), String> {
    grpc::logout(&command.host, &command.args[0]).await?;
    eprintln!("Logout Succeeded");
    Ok(())
}
