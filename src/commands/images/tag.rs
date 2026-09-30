use crate::cli::Invocation;
use crate::grpc;

pub async fn run(command: &Invocation) -> Result<(), String> {
    let source = &command.args[0];
    let destination = &command.args[1];
    if destination.to_ascii_lowercase().starts_with("sha256:")
        || (destination.len() == 64
            && destination
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')))
    {
        return Err(format!("{destination} is not a valid tag"));
    }

    grpc::tag_image(&command.host, source, destination).await
}
