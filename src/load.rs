use std::env;
use std::fs;
use std::path::Path;

use crate::cli::Invocation;
use crate::grpc;

pub async fn run(command: &Invocation) -> Result<(), String> {
    let input = command
        .options
        .iter()
        .rev()
        .find(|option| option.name == "input")
        .and_then(|option| option.value.as_deref())
        .ok_or("load requires -i or --input")?;
    let tag = command
        .options
        .iter()
        .rev()
        .find(|option| option.name == "tag")
        .and_then(|option| option.value.as_deref())
        .unwrap_or("");

    let path = Path::new(input);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|error| format!("cannot read current directory: {error}"))?
            .join(path)
    };
    let metadata = fs::metadata(&absolute)
        .map_err(|error| format!("cannot read {}: {error}", absolute.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a file", absolute.display()));
    }
    let file = absolute
        .to_str()
        .ok_or_else(|| format!("{} is not valid UTF-8", absolute.display()))?;

    grpc::load_image(&command.host, file, tag).await?;
    println!("Load image from \"{file}\" success");
    Ok(())
}
