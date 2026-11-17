use std::collections::HashMap;
use std::io::{self, Write};

use crate::cli::{parse_bool, Invocation};
use crate::grpc::networks;

fn column(output: &mut String, value: &str, width: usize) {
    output.push_str(value);
    output.extend(std::iter::repeat_n(
        ' ',
        width.saturating_sub(value.len()) + 1,
    ));
}
fn trim(value: &str) -> &str {
    value.trim_matches([' ', '\t', '\r', '\n', '\x0b', '\x0c'])
}

pub async fn run(command: &Invocation) -> Result<(), String> {
    let mut filters = HashMap::new();
    let mut quiet = false;
    for option in &command.options {
        match option.name {
            "quiet" => quiet = parse_bool(option.value.as_deref(), "quiet")?,
            "filter" => {
                let text = option.value.as_deref().unwrap_or_default();
                if text.is_empty() {
                    continue;
                }
                let (key, value) = text
                    .split_once('=')
                    .ok_or_else(|| format!("Bad format of filter '{text}', expected name=value"))?;
                filters.insert(trim(key).to_ascii_lowercase(), trim(value).to_owned());
            }
            _ => {}
        }
    }
    let mut entries = networks::list(&command.host, filters).await?;
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    let mut output = String::new();
    if quiet {
        for entry in entries {
            output.push_str(&entry.name);
            output.push('\n');
        }
    } else {
        let mut widths = [20, 15, 30];
        for entry in &entries {
            widths[0] = widths[0].max(entry.name.len());
            widths[1] = widths[1].max(if entry.version.is_empty() {
                6
            } else {
                entry.version.len()
            });
            // Native reserves a separator byte for every plugin, including the last.
            widths[2] = widths[2].max(entry.plugins.iter().map(|s| s.len() + 1).sum());
        }
        for (value, width) in ["NAME", "VERSION", "PLUGIN"].into_iter().zip(widths) {
            column(&mut output, value, width);
        }
        output.push('\n');
        for entry in entries {
            let version = if entry.version.is_empty() {
                "<none>"
            } else {
                &entry.version
            };
            let plugins = if entry.plugins.is_empty() {
                "<none>".into()
            } else {
                entry.plugins.join(",")
            };
            for (value, width) in [entry.name.as_str(), version, plugins.as_str()]
                .into_iter()
                .zip(widths)
            {
                column(&mut output, value, width);
            }
            output.push('\n');
        }
    }
    io::stdout()
        .lock()
        .write_all(output.as_bytes())
        .map_err(|e| e.to_string())
}
