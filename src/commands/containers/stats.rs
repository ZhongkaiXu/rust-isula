use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::time::Duration;

use crate::cli::{parse_bool, CliError, Invocation};
use crate::grpc::{
    self,
    proto::{ContainerInfo, StatsRequest},
};

fn size(bytes: u64) -> String {
    if bytes > 1024 * 1024 * 1024 {
        format!(
            "{}.{:02} GiB",
            (bytes >> 30) as u32,
            (bytes & ((1 << 30) - 1)) / 10_737_419
        )
    } else if bytes > 1024 * 1024 {
        let rounded = bytes + 5243;
        format!(
            "{}.{:02} MiB",
            rounded >> 20,
            ((rounded & ((1 << 20) - 1)) * 100) >> 20
        )
    } else if bytes > 1024 {
        let rounded = bytes + 5;
        format!(
            "{}.{:02} KiB",
            rounded >> 10,
            ((rounded & 1023) * 100) >> 10
        )
    } else {
        format!("{bytes}.00 B")
    }
}

fn cpu_percent(current: &ContainerInfo, previous: &[ContainerInfo]) -> f64 {
    let Some(old) = previous.iter().find(|old| old.id == current.id) else {
        return 0.0;
    };
    // Counter resets contribute zero, as in the native client.
    let system = current.cpu_system_use.saturating_sub(old.cpu_system_use);
    let cpu = current.cpu_use_nanos.saturating_sub(old.cpu_use_nanos);
    if system == 0 {
        0.0
    } else {
        cpu as f64 / system as f64 * f64::from(current.online_cpus) * 100.0
    }
}

fn frame(current: &[ContainerInfo], previous: &[ContainerInfo]) -> String {
    let mut output = String::from("\x1b[H\x1b[J\x1b[7m\x1b[1m");
    writeln!(
        output,
        "{:<16} {:<10} {:<26} {:<10} {:<26} {:<10}",
        "CONTAINER", "CPU %", "MEM USAGE / LIMIT", "MEM %", "BLOCK I / O", "PIDS"
    )
    .unwrap();
    output.push_str("\x1b[0m");
    for c in current {
        let id: String = c.id.chars().take(12).collect();
        let cpu = cpu_percent(c, previous);
        let mem = format!("{} / {}", size(c.workingset_bytes), size(c.mem_limit));
        let mem_percent = if c.mem_limit == 0 {
            0.0
        } else {
            c.workingset_bytes as f64 / c.mem_limit as f64 * 100.0
        };
        let io = format!("{} / {}", size(c.blkio_read), size(c.blkio_write));
        writeln!(
            output,
            "{id:<16} {cpu:<10.2} {mem:<26} {mem_percent:<10.2} {io:<26} {:<10}",
            c.pids_current
        )
        .unwrap();
    }
    output
}

fn original(current: &[ContainerInfo]) -> String {
    const WIDTHS: [usize; 13] = [16, 10, 10, 20, 20, 15, 15, 15, 15, 15, 15, 15, 40];
    fn row(output: &mut String, values: [String; 13]) {
        for (i, value) in values.iter().enumerate() {
            if i != 0 {
                output.push(' ');
            }
            write!(output, "{value:<width$}", width = WIDTHS[i]).unwrap();
        }
        output.push('\n');
    }
    let mut output = String::new();
    row(
        &mut output,
        [
            "ID",
            "PIDS",
            "Status",
            "CpuUseNanos",
            "CpuSystemUse",
            "OnlineCpus",
            "BlkioRead",
            "BlkioWrite",
            "MemUsed",
            "MemLimit",
            "KmemUsed",
            "CacheUsage",
            "NAME",
        ]
        .map(str::to_owned),
    );
    for c in current {
        row(
            &mut output,
            [
                c.id.chars().take(12).collect(),
                c.pids_current.to_string(),
                c.status.clone(),
                c.cpu_use_nanos.to_string(),
                c.cpu_system_use.to_string(),
                c.online_cpus.to_string(),
                c.blkio_read.to_string(),
                c.blkio_write.to_string(),
                c.mem_used.to_string(),
                c.mem_limit.to_string(),
                c.kmem_used.to_string(),
                c.cache.to_string(),
                c.name.clone(),
            ],
        );
    }
    output
}

fn failed(message: String) -> CliError {
    CliError {
        message,
        exit_code: 1,
    }
}

pub async fn run(command: &Invocation) -> Result<(), CliError> {
    let mut request = StatsRequest {
        containers: command.args.clone(),
        all: false,
    };
    let (mut no_stream, mut raw) = (false, false);
    for option in &command.options {
        match option.name {
            "all" => request.all = parse_bool(option.value.as_deref(), "all")?,
            "no-stream" => no_stream = parse_bool(option.value.as_deref(), "no-stream")?,
            "original" => raw = parse_bool(option.value.as_deref(), "original")?,
            _ => {}
        }
    }
    let mut previous = None;
    loop {
        let current = grpc::stats(&command.host, request.clone())
            .await
            .map_err(failed)?;
        let text = if raw {
            Some(original(&current))
        } else {
            previous.as_deref().map(|old| frame(&current, old))
        };
        if let Some(text) = text {
            let mut output = io::stdout().lock();
            output
                .write_all(text.as_bytes())
                .and_then(|_| output.flush())
                .map_err(|error| failed(error.to_string()))?;
            if raw || no_stream {
                return Ok(());
            }
        }
        previous = Some(current);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_follow_native_unit_boundaries_and_rounding() {
        for (bytes, expected) in [
            (0, "0.00 B"),
            (1024, "1024.00 B"),
            (1025, "1.00 KiB"),
            (1536, "1.50 KiB"),
            (1048576, "1024.00 KiB"),
            (1048577, "1.00 MiB"),
            (1073741824, "1024.00 MiB"),
            (1073741825, "1.00 GiB"),
            (1610612736, "1.49 GiB"),
        ] {
            assert_eq!(size(bytes), expected);
        }
    }

    #[test]
    fn cpu_matches_ids_and_handles_counter_resets() {
        let old = ContainerInfo {
            id: "a".into(),
            cpu_use_nanos: 100,
            cpu_system_use: 1000,
            ..Default::default()
        };
        let mut current = ContainerInfo {
            id: "a".into(),
            cpu_use_nanos: 150,
            cpu_system_use: 1200,
            online_cpus: 4,
            ..Default::default()
        };
        assert_eq!(
            cpu_percent(&current, &[ContainerInfo::default(), old.clone()]),
            100.0
        );
        assert_eq!(cpu_percent(&current, &[]), 0.0);
        current.cpu_use_nanos = 90;
        assert_eq!(cpu_percent(&current, std::slice::from_ref(&old)), 0.0);
        current.cpu_system_use = 900;
        assert_eq!(cpu_percent(&current, &[old]), 0.0);
    }
}
