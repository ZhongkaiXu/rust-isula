use serde_json::json;

use super::create::parse::{bytes, integer, scaled};
use crate::cli::{CliError, Invocation};
use crate::grpc::{self, proto};

fn error(message: impl ToString, exit_code: i32) -> CliError {
    CliError {
        message: message.to_string(),
        exit_code,
    }
}

pub fn build(command: &Invocation) -> Result<String, CliError> {
    let mut host = json!({});
    let mut restart = None;
    for option in &command.options {
        let value = option.value.as_deref().unwrap_or_default();
        let key = match option.name {
            "debug" => continue,
            "restart" => {
                restart = Some(value);
                continue;
            }
            "cpuset-cpus" | "cpuset-mems" => {
                let key = if option.name == "cpuset-cpus" {
                    "CpusetCpus"
                } else {
                    "CpusetMems"
                };
                if value.is_empty() {
                    host.as_object_mut().unwrap().remove(key);
                } else {
                    host[key] = json!(value);
                }
                continue;
            }
            "cpu-shares" => "CPUShares",
            "cpu-period" => "CPUPeriod",
            "cpu-quota" => "CPUQuota",
            "cpu-rt-period" => "CPURealtimePeriod",
            "cpu-rt-runtime" => "CPURealtimeRuntime",
            "cpus" => "NanoCpus",
            "blkio-weight" => "BlkioWeight",
            "memory" => "Memory",
            "memory-swap" => "MemorySwap",
            "memory-reservation" => "MemoryReservation",
            "kernel-memory" => "KernelMemory",
            _ => return Err(error("unsupported update option", 125)),
        };
        let result = match option.name {
            "memory-swap" if value == "-1" => Ok(-1),
            "memory" | "memory-swap" | "memory-reservation" | "kernel-memory" => bytes(value),
            "cpus" => {
                if !value.as_bytes().first().is_some_and(u8::is_ascii_digit) {
                    Err("invalid CPUs".into())
                } else {
                    scaled(value, 1_000_000_000)
                }
            }
            _ => integer(value),
        };
        let n = result.map_err(|e| error(e, 125))?;
        if option.name == "blkio-weight" && !(0..=65535).contains(&n) {
            return Err(error("blkio-weight out of range", 125));
        }
        if n == 0 {
            host.as_object_mut().unwrap().remove(key);
        } else {
            host[key] = json!(n);
        }
    }
    if let Some(policy) = restart {
        let (name, count) = policy.split_once(':').unwrap_or((policy, "0"));
        let count = integer(count).map_err(|e| error(e, 1))?;
        let count = i32::try_from(count).map_err(|_| error("restart count out of range", 1))?;
        host["RestartPolicy"] = json!({"Name":name});
        if count != 0 {
            host["RestartPolicy"]["MaximumRetryCount"] = json!(count);
        }
    }
    Ok(host.to_string())
}

pub async fn run(command: &Invocation) -> Result<(), CliError> {
    let hostconfig = build(command)?;
    let mut success = true;
    for name in &command.args {
        let result = if name.is_empty() {
            Err("Missing container name in the request".into())
        } else {
            grpc::update_container(
                &command.host,
                proto::UpdateRequest {
                    id: name.clone(),
                    hostconfig: hostconfig.clone(),
                },
            )
            .await
        };
        match result {
            Ok(()) => println!("{name}"),
            Err(message) => {
                eprintln!("risula update: {name}: {message}");
                success = false;
            }
        }
    }
    if success {
        Ok(())
    } else {
        Err(error("One or more containers failed to update", 1))
    }
}
