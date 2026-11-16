use std::io::{self, Write};

use crate::cli::{parse_bool, Invocation};
use crate::grpc::volumes;

fn column(output: &mut String, value: &str, width: usize) {
    output.push_str(value);
    output.extend(std::iter::repeat_n(
        ' ',
        width.saturating_sub(value.len()) + 1,
    ));
}

pub async fn run(command: &Invocation) -> Result<(), String> {
    let quiet = command
        .options
        .iter()
        .rev()
        .find(|o| o.name == "quiet")
        .map(|o| parse_bool(o.value.as_deref(), "quiet"))
        .transpose()?
        .unwrap_or(false);
    let entries = volumes::list(&command.host).await?;
    let mut output = String::new();
    if !quiet {
        column(&mut output, "DRIVER", 20);
        column(&mut output, "VOLUME NAME", 10);
        output.push('\n');
    }
    for entry in entries {
        if quiet {
            output.push_str(&entry.name);
        } else {
            column(&mut output, &entry.driver, 20);
            column(&mut output, &entry.name, 10);
        }
        output.push('\n');
    }
    io::stdout()
        .lock()
        .write_all(output.as_bytes())
        .map_err(|e| e.to_string())
}
