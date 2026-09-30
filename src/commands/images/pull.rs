use std::collections::HashMap;
use std::io::{self, IsTerminal};

use serde_json::Value;

use crate::cli::Invocation;
use crate::grpc;

pub async fn run(command: &Invocation) -> Result<(), String> {
    let name = &command.args[0];
    let show_progress = io::stdout().is_terminal();

    eprintln!("Image \"{name}\" pulling");
    let mut stream = grpc::pull_image(&command.host, name, show_progress).await?;
    let mut image_ref = None;
    let mut seen = HashMap::new();

    while let Some(message) = stream
        .message()
        .await
        .map_err(|error| format!("pull RPC failed: {error}"))?
    {
        if show_progress {
            for line in progress_lines(&message.progress_data, &mut seen) {
                println!("{line}");
            }
        }
        if !message.image_ref.is_empty() {
            image_ref = Some(message.image_ref);
        }
    }

    let image_ref = image_ref.ok_or("pull RPC ended without an image reference")?;
    eprintln!("Image \"{image_ref}\" pulled");
    Ok(())
}

fn progress_lines(data: &[u8], seen: &mut HashMap<String, (u64, u64)>) -> Vec<String> {
    let Ok(value) = serde_json::from_slice::<Value>(data) else {
        return Vec::new();
    };
    let Some(items) = value.get("progresses").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut lines = Vec::new();
    for item in items {
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        let current = item.get("current").and_then(Value::as_u64).unwrap_or(0);
        let total = item.get("total").and_then(Value::as_u64).unwrap_or(0);
        if seen.insert(id.to_string(), (current, total)) != Some((current, total)) {
            lines.push(format!("{id}: {current}/{total} bytes"));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_lines_show_changes_only() {
        let mut seen = HashMap::new();
        let frame = br#"{"progresses":[{"id":"layer-a","current":10,"total":20}]}"#;
        assert_eq!(progress_lines(frame, &mut seen), ["layer-a: 10/20 bytes"]);
        assert!(progress_lines(frame, &mut seen).is_empty());
        assert_eq!(
            progress_lines(
                br#"{"progresses":[{"id":"layer-a","current":20,"total":20}]}"#,
                &mut seen
            ),
            ["layer-a: 20/20 bytes"]
        );
        assert!(progress_lines(b"invalid json", &mut seen).is_empty());
    }
}
