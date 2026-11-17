use std::io::{self, Read, Write};

use crate::cli::{parse_bool, Invocation};
use crate::grpc::volumes;

pub async fn run(command: &Invocation) -> Result<(), String> {
    let force = command
        .options
        .iter()
        .rev()
        .find(|o| o.name == "force")
        .map(|o| parse_bool(o.value.as_deref(), "force"))
        .transpose()?
        .unwrap_or(false);
    if !force {
        print!("WARNING! This will remove all local volumes not used by at least one container.\nAre you sure you want to continue? [y/N]");
        io::stdout().flush().map_err(|e| e.to_string())?;
        // Native getchar checks only the first byte, including when input is piped.
        let mut answer = [0];
        if io::stdin().read_exact(&mut answer).is_err() || !matches!(answer[0], b'y' | b'Y') {
            return Ok(());
        }
    }
    let names = volumes::prune(&command.host).await?;
    let mut stdout = io::stdout().lock();
    for name in names {
        writeln!(stdout, "{name}").map_err(|e| e.to_string())?;
    }
    Ok(())
}
