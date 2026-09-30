use std::env;
use std::path::Path;

use crate::cli::Invocation;
use crate::grpc;

pub async fn run(command: &Invocation) -> Result<(), String> {
    let file = Path::new(&command.args[0]);
    let tag = &command.args[1];
    if tag.to_ascii_lowercase().starts_with("sha256:")
        || (tag.len() == 64
            && tag
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')))
    {
        return Err(format!("{tag} is not a valid image name"));
    }

    let absolute = if file.is_absolute() {
        file.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|error| format!("cannot read current directory: {error}"))?
            .join(file)
    };
    let file = absolute
        .to_str()
        .ok_or_else(|| format!("{} is not valid UTF-8", absolute.display()))?;

    let id = grpc::import_image(&command.host, file, tag).await?;
    if !id.is_empty() {
        println!("sha256:{id}");
    }
    Ok(())
}
