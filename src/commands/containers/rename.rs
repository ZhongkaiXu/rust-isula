use crate::cli::Invocation;
use crate::grpc;

pub async fn run(command: &Invocation) -> Result<(), String> {
    let old_name = &command.args[0];
    let new_name = &command.args[1];
    if old_name.is_empty() || new_name.is_empty() {
        return Err("container old name and new name cannot be empty".into());
    }
    grpc::rename_container(&command.host, old_name, new_name).await
}
