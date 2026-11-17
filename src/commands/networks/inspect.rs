use serde_json::Value;

use crate::cli::Invocation;
use crate::commands::containers::inspect::{fields, render, select};
use crate::grpc::networks;

pub async fn run(command: &Invocation) -> Result<(), String> {
    let format = command
        .options
        .iter()
        .rev()
        .find(|o| o.name == "format")
        .map(|o| o.value.as_deref().unwrap_or_default());
    let field = if let Some(text) = format {
        let mut parsed = fields(text)?;
        if parsed.len() != 1 {
            return Err("network inspect accepts one format expression".into());
        }
        let mut field = parsed.remove(0);
        // Native extracts the path using only space and '}' as delimiters.
        let path = text
            .split_once('.')
            .unwrap()
            .1
            .split([' ', '}'])
            .next()
            .unwrap();
        field.path = path.split('.').map(str::to_owned).collect();
        Some(field)
    } else {
        None
    };
    let mut values = Vec::new();
    let mut failure = None;
    for name in &command.args {
        let result = async {
            let text = networks::inspect(&command.host, name).await?;
            let value: Value = serde_json::from_str(&text)
                .map_err(|e| format!("invalid inspect response: {e}"))?;
            if let Some(field) = &field {
                select(&value, &field.path)
                    .cloned()
                    .ok_or_else(|| format!("map has no entry for .{}", field.path.join(".")))
            } else {
                Ok(value)
            }
        }
        .await;
        match result {
            Ok(value) => values.push(value),
            Err(error) => {
                failure = Some(format!(
                    "{error}\nInspect error, cannot find such network: {name}"
                ));
                break;
            }
        }
    }
    let mut output = String::new();
    if let Some(field) = &field {
        for value in values {
            render(&mut output, &value, 0, field.json);
            output.push('\n');
        }
    } else {
        // Network inspect uses unquoted string values even without --format.
        render(&mut output, &Value::Array(values), 0, false);
        output.push('\n');
    }
    print!("{output}");
    failure.map_or(Ok(()), Err)
}
