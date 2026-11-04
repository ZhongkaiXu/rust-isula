use std::io::{self, Write};

use crate::{cli::Invocation, grpc};

pub async fn run(command: &Invocation) -> Result<(), String> {
    let response =
        grpc::top::processes(&command.host, &command.args[0], command.args[1..].to_vec()).await?;
    // The upstream client suppresses the header when there are no process rows.
    if !response.processes.is_empty() {
        let mut output = io::stdout().lock();
        if !response.titles.is_empty() {
            output
                .write_all(&response.titles)
                .and_then(|_| output.write_all(b"\n"))
                .map_err(|error| error.to_string())?;
        }
        for process in response.processes {
            output
                .write_all(&process)
                .and_then(|_| output.write_all(b"\n"))
                .map_err(|error| error.to_string())?;
        }
    }
    if response.cc != 0 {
        return Err(response.errmsg);
    }
    Ok(())
}
