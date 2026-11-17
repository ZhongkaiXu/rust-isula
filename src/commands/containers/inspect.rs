use std::fmt::Write;

use serde_json::Value;

use crate::cli::{CliError, Invocation};
use crate::grpc;

pub(crate) struct Field {
    pub(crate) path: Vec<String>,
    pub(crate) json: bool,
}

pub(crate) fn fields(mut text: &str) -> Result<Vec<Field>, String> {
    let mut fields = Vec::new();
    while !text.trim().is_empty() {
        text = text
            .trim_start()
            .strip_prefix('{')
            .ok_or("expected {{.Field}}")?;
        text = text
            .trim_start()
            .strip_prefix('{')
            .ok_or("expected {{.Field}}")?;
        let end = text.find('}').ok_or("expected closing }}")?;
        let mut expression = text[..end].split_whitespace();
        let first = expression.next().ok_or("empty inspect format")?;
        let json = first == "json";
        let path = if json {
            expression.next().ok_or("json requires a field")?
        } else {
            first
        };
        if expression.next().is_some() {
            return Err("inspect format accepts one field per expression".into());
        }
        let path = path.strip_prefix('.').ok_or("expected .Field")?;
        let path: Vec<_> = path.split('.').map(str::to_owned).collect();
        if path.iter().any(|part| {
            part.is_empty() || !part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        }) {
            return Err("invalid inspect field path".into());
        }
        fields.push(Field { path, json });
        text = text[end + 1..]
            .trim_start()
            .strip_prefix('}')
            .ok_or("expected closing }}")?;
    }
    if fields.is_empty() {
        return Err("empty inspect format".into());
    }
    Ok(fields)
}

pub(crate) fn select<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let Some((key, rest)) = path.split_first() else {
        return Some(value);
    };
    match value {
        Value::Object(map) => select(map.get(key)?, rest),
        // Upstream selects the first array object containing the requested path.
        Value::Array(items) => items.iter().find_map(|item| select(item.get(key)?, rest)),
        _ => None,
    }
}

fn string(output: &mut String, value: &str, quoted: bool) {
    if quoted {
        output.push('"');
    }
    for c in value.chars() {
        match c {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{8}' => output.push_str("\\b"),
            '\u{c}' => output.push_str("\\f"),
            c if c < ' ' => write!(output, "\\u{:04X}", c as u32).unwrap(),
            c => output.push(c),
        }
    }
    if quoted {
        output.push('"');
    }
}

pub(crate) fn render(output: &mut String, value: &Value, indent: usize, json: bool) {
    match value {
        Value::Object(map) => {
            output.push('{');
            for (index, (key, value)) in map.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write!(output, "\n{:width$}", "", width = indent + 4).unwrap();
                string(output, key, true);
                output.push_str(": ");
                render(output, value, indent + 4, json);
            }
            if !map.is_empty() {
                write!(output, "\n{:indent$}", "").unwrap();
            }
            output.push('}');
        }
        Value::Array(items) => {
            output.push('[');
            for (index, value) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write!(output, "\n{:width$}", "", width = indent + 4).unwrap();
                render(output, value, indent + 4, json);
            }
            if !items.is_empty() {
                write!(output, "\n{:indent$}", "").unwrap();
            }
            output.push(']');
        }
        Value::String(value) => string(output, value, json),
        _ => write!(output, "{value}").unwrap(),
    }
}

fn failed(message: String) -> CliError {
    CliError {
        message,
        exit_code: 1,
    }
}

pub async fn run(command: &Invocation) -> Result<(), CliError> {
    if command.args.is_empty() || command.args.len() >= 1000 {
        return Err(CliError {
            message: "inspect requires between 1 and 999 objects".into(),
            exit_code: 125,
        });
    }
    let mut timeout = 120;
    let mut format = None;
    for option in &command.options {
        match option.name {
            "time" => {
                timeout =
                    super::parse_time(option.value.as_deref().unwrap_or("")).map_err(|message| {
                        CliError {
                            message,
                            exit_code: 125,
                        }
                    })?
            }
            "format" => format = Some(option.value.as_deref().unwrap_or("")),
            _ => {}
        }
    }
    let fields = format.map(fields).transpose().map_err(failed)?;
    let mut objects = Vec::new();
    let mut output = String::new();
    for name in &command.args {
        let text = grpc::inspect_object(&command.host, name, fields.is_some(), timeout)
            .await
            .map_err(failed)?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|error| failed(format!("invalid inspect response: {error}")))?;
        if let Some(fields) = &fields {
            for field in fields {
                let selected = select(&value, &field.path).ok_or_else(|| {
                    failed(format!("map has no entry for .{}", field.path.join(".")))
                })?;
                render(&mut output, selected, 0, field.json);
                output.push('\n');
            }
        } else {
            objects.push(value);
        }
    }
    if fields.is_none() {
        render(&mut output, &Value::Array(objects), 0, true);
        output.push('\n');
    }
    // Keep stdout empty if any object or field lookup fails.
    print!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_paths_and_array_lookup() {
        let parsed = fields(" { {json .Items.Name} } {{.Id}} ").unwrap();
        assert!(parsed[0].json);
        let value: Value =
            serde_json::from_str(r#"{"Items":[{"Other":1},{"Name":"found"}],"Id":"abc"}"#).unwrap();
        assert_eq!(select(&value, &parsed[0].path).unwrap(), "found");
        for invalid in [
            "",
            "text",
            "{{.}}",
            "{{json}}",
            "{{.A..B}}",
            "{{.A}}|{{.B}}",
            "{{index .A 0}}",
        ] {
            assert!(fields(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn output_preserves_order_indentation_and_string_escapes() {
        let value: Value = serde_json::from_str(r#"{"z":["a\nb",null],"a":{}}"#).unwrap();
        let mut output = String::new();
        render(&mut output, &value, 0, false);
        assert_eq!(
            output,
            "{\n    \"z\": [\n        a\\nb,\n        null\n    ],\n    \"a\": {}\n}"
        );
        output.clear();
        string(&mut output, "\u{1b}\"\\", true);
        assert_eq!(output, "\"\\u001B\\\"\\\\\"");
    }
}
