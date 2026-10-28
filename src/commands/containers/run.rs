use crate::cli::{parse_bool, CliError, Invocation};
use crate::grpc::{proto, start as rpc};

use super::{console, create, start};

pub async fn run(command: &Invocation) -> Result<i32, CliError> {
    let (request, policy) = create::build(command)?;
    let (mut detach, mut interactive, mut tty, mut auto_remove) = (false, false, false, false);
    for option in &command.options {
        let target = match option.name {
            "detach" => &mut detach,
            "interactive" => &mut interactive,
            "tty" => &mut tty,
            "rm" => &mut auto_remove,
            _ => continue,
        };
        *target = parse_bool(option.value.as_deref(), option.name)?;
    }
    let id = create::create(&command.host, request, &policy).await?;
    if detach {
        println!("{id}");
    }
    let failure = |err: std::io::Error| CliError {
        message: err.to_string(),
        exit_code: 1,
    };
    let _terminal = console::Terminal::new(tty && interactive && !detach).map_err(failure)?;
    let input = if interactive && !detach {
        Some(console::Input::new().map_err(failure)?)
    } else {
        None
    };
    let execute = async {
        if command.host.starts_with("tcp://") {
            start::start_result(
                rpc::remote_start_with_input(&command.host, &id, input, !detach).await,
                true,
            )?;
            if detach {
                return Ok(0);
            }
            rpc::wait_for(&command.host, &id, auto_remove)
                .await
                .map(|code| code as i32)
                .map_err(|message| CliError {
                    message,
                    exit_code: 125,
                })
        } else if detach {
            start::start_result(
                rpc::start_container(
                    &command.host,
                    proto::StartRequest {
                        id: id.clone(),
                        ..Default::default()
                    },
                )
                .await,
                false,
            )?;
            Ok(0)
        } else {
            start::local_start(&command.host, &id, input, tty, auto_remove).await
        }
    };
    tokio::select! {
        result = execute => result,
        _ = console::resize(&command.host, &id, tty) => unreachable!(),
    }
}
