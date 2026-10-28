use super::parse::*;
use crate::cli::{parse_bool, CliError, Invocation};
use crate::grpc::proto::CreateRequest;
use serde_json::{json, Value};

fn error(message: impl Into<String>, exit_code: i32) -> CliError {
    CliError {
        message: message.into(),
        exit_code,
    }
}
fn push(object: &mut Value, key: &str, value: Value) {
    if object.get(key).is_none() {
        object[key] = json!([]);
    }
    object[key].as_array_mut().unwrap().push(value);
}
fn set_number(object: &mut Value, key: &str, value: i64) {
    if value == 0 {
        object.as_object_mut().unwrap().remove(key);
    } else {
        object[key] = json!(value);
    }
}
fn set_flag(object: &mut Value, key: &str, value: bool) {
    if value {
        object[key] = json!(true);
    } else {
        object.as_object_mut().unwrap().remove(key);
    }
}
fn map_entry(object: &mut Value, key: &str, entry: &str, value: &str) {
    if object.get(key).is_none() {
        object[key] = json!({});
    }
    object[key][entry] = json!(value);
}
fn read_file(path: &str) -> Result<String, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let meta = file.metadata().map_err(|e| format!("{path}: {e}"))?;
    const LIMIT: u64 = 10 * 1024 * 1024;
    if !meta.is_file() || meta.len() > LIMIT {
        return Err(format!("invalid or oversized file: {path}"));
    }
    let mut text = String::new();
    file.take(LIMIT + 1)
        .read_to_string(&mut text)
        .map_err(|e| format!("{path}: {e}"))?;
    if text.len() as u64 > LIMIT {
        return Err(format!("oversized file: {path}"));
    }
    Ok(text)
}
fn lines(path: &str, comments: bool) -> Result<Vec<String>, String> {
    Ok(read_file(path)?
        .lines()
        .map(|s| if comments { s } else { s.trim() })
        .filter(|s| !s.is_empty() && !(comments && s.starts_with('#')))
        .map(str::to_owned)
        .collect())
}
fn env(value: &str) -> Result<Option<String>, String> {
    let key = value.split('=').next().unwrap_or("");
    if key.is_empty() {
        return Err("environment variable name cannot be empty".into());
    }
    Ok(if value.contains('=') {
        Some(value.into())
    } else {
        std::env::var(value).ok().map(|v| format!("{value}={v}"))
    })
}

pub(crate) fn build(command: &Invocation) -> Result<(CreateRequest, String), CliError> {
    let option_error = |e| error(e, 125);
    let pack_error = |e| error(e, 1);
    let mut host = json!({"RestartPolicy":{"Name":"no"}});
    let mut container = json!({"Annotations":{}});
    let mut request = CreateRequest {
        image: command
            .args
            .first()
            .ok_or_else(|| error("create requires an image", 125))?
            .clone(),
        ..Default::default()
    };
    if command.args.len() > 1 {
        container["Cmd"] = json!(command.args[1..]);
    }
    let mut detach = false;
    let mut policy = "missing".to_string();
    let mut rootfs = None;
    let mut network = None;
    let mut restart = "no".to_string();
    let mut health = json!({});
    let mut health_cmd = String::new();
    let mut no_health = false;
    let mut publish_all = false;
    let mut exposes = Vec::new();
    let mut expose = json!({});
    let mut bindings = json!({});
    let mut envs = Vec::new();
    let mut labels = Vec::new();
    let mut env_files = Vec::new();
    let mut label_files = Vec::new();
    // Repeated scalar options use the last value; list options retain their order.
    for option in &command.options {
        let key = option.name;
        let v = option.value.as_deref().unwrap_or("");
        let bool_value = || parse_bool(option.value.as_deref(), key).map_err(option_error);
        let num = || integer(v).map_err(option_error);
        let size = || {
            if key == "memory-swap" && v == "-1" {
                Ok(-1)
            } else {
                bytes(v).map_err(option_error)
            }
        };
        match key {
            "debug" => {}
            "detach" => detach = bool_value()?,
            "name" => request.id = v.into(),
            "runtime" => request.runtime = v.into(),
            "external-rootfs" => rootfs = Some(v.to_string()),
            "pull" => {
                if !matches!(v, "always" | "missing" | "never") {
                    return Err(error("--pull must be always, missing or never", 125));
                }
                policy = v.into();
            }
            "hostname" | "user" | "workdir" | "stop-signal" | "log-driver" => {
                let field = match key {
                    "hostname" => "Hostname",
                    "user" => "User",
                    "workdir" => "WorkingDir",
                    "stop-signal" => "StopSignal",
                    _ => "LogDriver",
                };
                if key == "log-driver" && !matches!(v, "syslog" | "json-file") {
                    return Err(error("invalid log driver", 125));
                }
                container[field] = json!(v);
            }
            "entrypoint" => {
                container.as_object_mut().unwrap().remove("Entrypoint");
                if !v.is_empty() {
                    container["Entrypoint"] = json!([v]);
                }
            }
            "env" => envs.push(v.to_string()),
            "label" => labels.push(v.to_string()),
            "env-file" => env_files.push(v),
            "label-file" => label_files.push(v),
            "annotation" => {
                let (k, v) = v.split_once('=').unwrap_or((v, ""));
                container["Annotations"][k] = json!(v);
            }
            "log-opt" => log_option(v, &mut container["Annotations"]).map_err(option_error)?,
            "tty" => set_flag(&mut container, "Tty", bool_value()?),
            "interactive" => {
                let b = bool_value()?;
                set_flag(&mut container, "OpenStdin", b);
                set_flag(&mut container, "AttachStdin", b);
            }
            "privileged" | "read-only" | "rm" | "system-container" | "no-pivot"
            | "oom-kill-disable" => {
                let b = bool_value()?;
                let field = match key {
                    "privileged" => "Privileged",
                    "read-only" => "ReadonlyRootfs",
                    "rm" => "AutoRemove",
                    "system-container" => "SystemContainer",
                    "no-pivot" => "NoPivotRoot",
                    _ => "OomKillDisable",
                };
                set_flag(&mut host, field, b);
                if key == "rm" {
                    set_flag(&mut host, "AutoRemoveBak", b);
                }
                if key == "system-container" {
                    set_flag(&mut container, "SystemContainer", b);
                }
            }
            "net" | "network" => network = Some(v.to_string()),
            "pid" | "ipc" | "uts" | "cgroupns" | "cgroup-parent" | "cpuset-cpus"
            | "cpuset-mems" | "env-target-file" | "hook-spec" | "user-remap" | "ip" => {
                let field = match key {
                    "pid" => "PidMode",
                    "ipc" => "IpcMode",
                    "uts" => "UTSMode",
                    "cgroupns" => "CgroupnsMode",
                    "cgroup-parent" => "CgroupParent",
                    "cpuset-cpus" => "CpusetCpus",
                    "cpuset-mems" => "CpusetMems",
                    "env-target-file" => "EnvTargetFile",
                    "hook-spec" => "HookSpec",
                    "user-remap" => "UserRemap",
                    _ => "IP",
                };
                host[field] = json!(v);
            }
            "ns-change-opt" => container["NsChangeOpt"] = json!(v),
            "restart" => restart = v.into(),
            "memory" | "memory-swap" | "memory-reservation" | "kernel-memory" | "shm-size" => {
                let field = match key {
                    "memory" => "Memory",
                    "memory-swap" => "MemorySwap",
                    "memory-reservation" => "MemoryReservation",
                    "kernel-memory" => "KernelMemory",
                    _ => "ShmSize",
                };
                let n = size()?;
                if n < 0 && !(key == "memory-swap" && v == "-1") {
                    return Err(error(format!("invalid --{key}"), 125));
                }
                set_number(&mut host, field, n);
            }
            "cpus" => {
                let n = scaled(v, 1_000_000_000).map_err(option_error)?;
                if n < 0 || !v.as_bytes().first().is_some_and(u8::is_ascii_digit) {
                    return Err(error("invalid CPUs", 125));
                }
                set_number(&mut host, "NanoCpus", n);
            }
            "cpu-shares" | "cpu-period" | "cpu-quota" | "cpu-rt-period" | "cpu-rt-runtime"
            | "pids-limit" | "files-limit" | "oom-score-adj" | "memory-swappiness"
            | "blkio-weight" => {
                let n = if key == "memory-swappiness" && v != "-1" {
                    bytes(v).map_err(option_error)?
                } else {
                    num()?
                };
                let field = match key {
                    "cpu-shares" => "CPUShares",
                    "cpu-period" => "CPUPeriod",
                    "cpu-quota" => "CPUQuota",
                    "cpu-rt-period" => "CPURealtimePeriod",
                    "cpu-rt-runtime" => "CPURealtimeRuntime",
                    "pids-limit" => "PidsLimit",
                    "files-limit" => "FilesLimit",
                    "oom-score-adj" => "OomScoreAdj",
                    "memory-swappiness" => "MemorySwappiness",
                    _ => "BlkioWeight",
                };
                if (key == "memory-swappiness" && !(-1..=100).contains(&n))
                    || (key == "blkio-weight" && !(0..=u16::MAX as i64).contains(&n))
                {
                    return Err(error(format!("--{key} out of range"), 125));
                }
                if key == "memory-swappiness" {
                    if n == -1 {
                        host.as_object_mut().unwrap().remove(field);
                    } else {
                        host[field] = json!(n);
                    }
                } else {
                    set_number(&mut host, field, n);
                }
            }
            "add-host" | "dns" | "dns-opt" | "dns-search" | "cap-add" | "cap-drop"
            | "group-add" | "device-cgroup-rule" | "volumes-from" | "volume" => {
                let field = match key {
                    "add-host" => "ExtraHosts",
                    "dns" => "Dns",
                    "dns-opt" => "DnsOptions",
                    "dns-search" => "DnsSearch",
                    "cap-add" => "CapAdd",
                    "cap-drop" => "CapDrop",
                    "group-add" => "GroupAdd",
                    "device-cgroup-rule" => "DeviceCgroupRules",
                    "volumes-from" => "VolumesFrom",
                    _ => "Binds",
                };
                if key == "volume" {
                    volume(v).map_err(option_error)?;
                }
                if matches!(key, "cap-add" | "cap-drop") {
                    capability(v).map_err(pack_error)?;
                }
                push(&mut host, field, json!(v));
            }
            "tmpfs" => map_entry(&mut host, "Tmpfs", v, ""),
            "mount" => push(&mut host, "Mounts", mount(v).map_err(option_error)?),
            "device" => push(&mut host, "Devices", device(v).map_err(option_error)?),
            "security-opt" => push(
                &mut host,
                "SecurityOpt",
                json!(security(v).map_err(pack_error)?),
            ),
            "storage-opt" | "sysctl" => {
                let (k, v) = pair(v).map_err(if key == "sysctl" {
                    option_error
                } else {
                    pack_error
                })?;
                if key == "sysctl" {
                    sysctl(k).map_err(option_error)?;
                }
                map_entry(
                    &mut host,
                    if key == "sysctl" {
                        "Sysctls"
                    } else {
                        "StorageOpt"
                    },
                    k,
                    v,
                );
            }
            "ulimit" => {
                let limit = ulimit(v).map_err(pack_error)?;
                if let Some(a) = host.get_mut("Ulimits").and_then(Value::as_array_mut) {
                    a.retain(|x| x["Name"] != limit["Name"]);
                }
                push(&mut host, "Ulimits", limit);
            }
            "blkio-weight-device"
            | "device-read-bps"
            | "device-write-bps"
            | "device-read-iops"
            | "device-write-iops" => {
                let (path, n) = v
                    .split_once(':')
                    .ok_or_else(|| error("expected device:value", 1))?;
                absolute(path).map_err(pack_error)?;
                let n = if key.ends_with("bps") {
                    bytes(n).map(|n| n as u64)
                } else {
                    unsigned(n)
                }
                .map_err(pack_error)?;
                if key == "blkio-weight-device" && n != 0 && !(10..=1000).contains(&n) {
                    return Err(error("device weight out of range", 1));
                }
                let field = match key {
                    "blkio-weight-device" => "BlkioWeightDevice",
                    "device-read-bps" => "BlkioDeviceReadBps",
                    "device-write-bps" => "BlkioDeviceWriteBps",
                    "device-read-iops" => "BlkioDeviceReadIops",
                    _ => "BlkioDeviceWriteIops",
                };
                let mut d = json!({"Path":path});
                if n != 0 {
                    d[if key == "blkio-weight-device" {
                        "Weight"
                    } else {
                        "Rate"
                    }] = json!(n);
                }
                push(&mut host, field, d);
            }
            "hugetlb-limit" => {
                let (page, limit) = v.split_once(':').unwrap_or(("", v));
                let limit = bytes(limit).map_err(option_error)?;
                if limit < 0 {
                    return Err(error("negative hugepage limit", 1));
                }
                let page = if page.is_empty() {
                    String::new()
                } else {
                    human_size(bytes(page).map_err(pack_error)?)
                };
                let mut d = json!({"PageSize":page});
                set_number(&mut d, "Limit", limit);
                push(&mut host, "Hugetlbs", d);
            }
            "host-channel" => host["HostChannel"] = host_channel(v).map_err(pack_error)?,
            "health-cmd" => health_cmd = v.into(),
            "health-interval" | "health-timeout" | "health-start-period" => {
                let field = match key {
                    "health-interval" => "Interval",
                    "health-timeout" => "Timeout",
                    _ => "StartPeriod",
                };
                set_number(&mut health, field, duration(v).map_err(option_error)?);
            }
            "health-retries" => {
                let n = num()?;
                if n < i32::MIN as i64 || n > i32::MAX as i64 {
                    return Err(error("invalid health retries", 125));
                }
                set_number(&mut health, "Retries", n);
            }
            "health-exit-on-unhealthy" => set_flag(&mut health, "ExitOnUnhealthy", bool_value()?),
            "no-healthcheck" => no_health = bool_value()?,
            "publish-all" => publish_all = bool_value()?,
            "expose" => exposes.push(v),
            "publish" => publish(v, &mut expose, &mut bindings).map_err(pack_error)?,
            _ => return Err(error(format!("unhandled create option: {key}"), 125)),
        }
    }
    if command.options.iter().any(|o| o.name == "name")
        && (request.id.len() < 2 || !name(&request.id))
    {
        return Err(error("invalid container name", 125));
    }
    if let Some(s) = container["Hostname"].as_str() {
        if s.len() > 64
            || s.split('.').any(|part| {
                part.is_empty()
                    || !part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    || !part.as_bytes()[0].is_ascii_alphanumeric()
                    || !part.as_bytes()[part.len() - 1].is_ascii_alphanumeric()
            })
        {
            return Err(error("invalid hostname", 125));
        }
    }
    if rootfs.is_none() && matches!(request.image.as_str(), "none" | "none:latest") {
        rootfs = Some(std::env::var("IMAGE_NONE_PATH").unwrap_or_else(|_| "/dev/ram0".into()));
    }
    if let Some(path) = rootfs {
        request.rootfs = std::fs::canonicalize(&path)
            .map_err(|e| error(format!("rootfs {path}: {e}"), 125))?
            .to_str()
            .ok_or_else(|| error("rootfs path is not UTF-8", 125))?
            .into();
    }
    if request.image.is_empty() && request.rootfs.is_empty() {
        return Err(error("image and rootfs cannot both be empty", 1));
    }
    validate_host(
        &mut host,
        &mut container,
        network.as_deref(),
        &request.rootfs,
    )
    .map_err(option_error)?;
    if host["AutoRemove"] == true && restart != "no" {
        return Err(error("conflicting options: --restart and --rm", 125));
    }
    let (r, count) = restart.split_once(':').unwrap_or((&restart, "0"));
    let count = integer(count).map_err(pack_error)?;
    let count = i32::try_from(count).map_err(|_| error("restart count out of range", 1))?;
    host["RestartPolicy"] = json!({"Name":r});
    set_number(
        &mut host["RestartPolicy"],
        "MaximumRetryCount",
        count as i64,
    );
    for path in env_files {
        for v in lines(path, true).map_err(pack_error)? {
            if let Some(v) = env(&v).map_err(pack_error)? {
                push(&mut container, "Env", json!(v));
            }
        }
    }
    for v in envs {
        if let Some(v) = env(&v).map_err(pack_error)? {
            push(&mut container, "Env", json!(v));
        }
    }
    let mut all_labels = Vec::new();
    for path in label_files {
        all_labels.extend(lines(path, false).map_err(pack_error)?);
    }
    all_labels.extend(labels);
    for v in all_labels {
        let (k, v) = v.split_once('=').unwrap_or((&v, ""));
        if k.is_empty() {
            return Err(error("empty label key", 1));
        }
        map_entry(&mut container, "Labels", k, v);
    }
    let have_health = !health_cmd.is_empty()
        || ["Interval", "Timeout", "StartPeriod", "Retries"]
            .iter()
            .any(|k| health.get(k).is_some());
    if no_health {
        if have_health {
            return Err(error(
                "--no-healthcheck conflicts with --health-* options",
                1,
            ));
        }
        container["Healthcheck"] = json!({"Test":["NONE"]});
    } else if have_health {
        if health_cmd.is_empty() {
            return Err(error("--health-cmd required", 1));
        }
        health["Test"] = json!(["CMD-SHELL", health_cmd]);
        container["Healthcheck"] = health;
    }
    if publish_all {
        for v in exposes {
            let (text, proto) = v.split_once('/').unwrap_or((v, "tcp"));
            if !matches!(proto, "tcp" | "udp" | "sctp") {
                return Err(error("invalid expose protocol", 1));
            }
            for p in ports(text).map_err(pack_error)? {
                expose[format!("{p}/{proto}")] = json!({});
            }
        }
    }
    if !expose.as_object().unwrap().is_empty() {
        container["ExposedPorts"] = expose;
    }
    if !bindings.as_object().unwrap().is_empty() {
        host["PortBindings"] = bindings;
    }
    if command.name == "run" {
        let attach_stdin = !detach && container["OpenStdin"] == true;
        set_flag(&mut container, "AttachStdin", attach_stdin);
        set_flag(&mut container, "AttachStdout", !detach);
        set_flag(&mut container, "AttachStderr", !detach);
    }
    request.hostconfig = host.to_string();
    request.customconfig = container.to_string();
    Ok((request, policy))
}

fn validate_host(
    host: &mut Value,
    container: &mut Value,
    network: Option<&str>,
    rootfs: &str,
) -> Result<(), String> {
    for v in host["Dns"].as_array().into_iter().flatten() {
        v.as_str()
            .unwrap()
            .parse::<std::net::Ipv4Addr>()
            .map_err(|_| "invalid DNS address")?;
    }
    for v in host["ExtraHosts"].as_array().into_iter().flatten() {
        let (name, ip) = v
            .as_str()
            .unwrap()
            .split_once(':')
            .ok_or("expected host:ip")?;
        if name.is_empty() {
            return Err("empty extra host name".into());
        }
        ip.parse::<std::net::Ipv4Addr>()
            .map_err(|_| "invalid extra host IP")?;
    }
    let bridge =
        network.is_some_and(|n| !matches!(n, "none" | "host") && !n.starts_with("container:"));
    if let Some(n) = network {
        if bridge {
            let mut networks = Vec::new();
            for network in n.split(',').filter(|s| !s.is_empty()) {
                if !networks.contains(&network) {
                    networks.push(network);
                }
            }
            if n.len() > (128 + 1) * 1024 - 1 {
                return Err("invalid bridge network names".into());
            }
            host["NetworkMode"] = json!("bridge");
            host["BridgeNetwork"] = json!(networks);
        } else {
            host["NetworkMode"] = json!(n);
        }
    }
    if let Some(ip) = host["IP"].as_str() {
        if !bridge || network.unwrap_or("").contains(',') {
            return Err("static IP requires a single bridge network".into());
        }
        ip.parse::<std::net::IpAddr>()
            .map_err(|_| "invalid static IP")?;
    }
    if let Some(remap) = host["UserRemap"].as_str() {
        if host["Privileged"] == true || host["SystemContainer"] != true || rootfs.is_empty() {
            return Err("--user-remap requires a system container with external rootfs".into());
        }
        let ids: Vec<_> = remap.split(':').collect();
        if ids.len() != 3
            || ids.iter().any(|s| s.parse::<u32>().is_err())
            || ids[2].parse::<u32>().is_ok_and(|n| n == 0 || n > 65535)
        {
            return Err("invalid user remap".into());
        }
    }
    if let Some(path) = host["HookSpec"].as_str() {
        absolute(path)?;
        let m = std::fs::metadata(path).map_err(|e| format!("hook spec: {e}"))?;
        if !m.is_file() || m.len() > 10 * 1024 * 1024 {
            return Err("hook spec must be a regular file no larger than 10MB".into());
        }
    }
    if let Some(path) = host["EnvTargetFile"].as_str() {
        absolute(path)?;
        if rootfs.is_empty() {
            eprintln!("external rootfs not specified");
        } else {
            let base = std::path::Path::new(rootfs);
            let target = base.join(absolute(path)?.trim_start_matches('/'));
            let mut ancestor = target.as_path();
            while !ancestor.exists() {
                ancestor = ancestor.parent().ok_or("invalid env target path")?;
            }
            if !std::fs::canonicalize(ancestor)
                .map_err(|e| e.to_string())?
                .starts_with(base)
            {
                return Err("env target path escapes external rootfs".into());
            }
            if std::fs::metadata(&target).is_ok_and(|m| m.len() > 10 * 1024 * 1024) {
                return Err("env target file exceeds 10MB".into());
            }
        }
    }
    if let Some(ns) = container["NsChangeOpt"].as_str() {
        if host["SystemContainer"] != true {
            return Err("--ns-change-opt requires a system container".into());
        }
        let parts: Vec<_> = ns.split(',').collect();
        if parts.len() > 2 || parts.iter().any(|s| !matches!(*s, "net" | "ipc")) {
            return Err("invalid ns-change-opt".into());
        }
        if host["Privileged"] == true {
            container.as_object_mut().unwrap().remove("NsChangeOpt");
        } else {
            let mut files = Vec::new();
            if parts.contains(&"net") {
                files.push("/proc/sys/net".to_string());
            }
            if parts.contains(&"ipc") {
                for name in [
                    "shmmax",
                    "shmmni",
                    "shmall",
                    "shm_rmid_forced",
                    "msgmax",
                    "msgmni",
                    "msgmnb",
                    "sem",
                ] {
                    files.push(format!("/proc/sys/kernel/{name}"));
                }
                files.push("/proc/sys/fs/mqueue".into());
            }
            host["NsChangeFiles"] = json!(files);
        }
    }
    if host["OomKillDisable"] == true && host.get("Memory").is_none() {
        eprintln!("WARNING: disabling the OOM killer without a memory limit may be dangerous");
    }
    Ok(())
}

fn capability(value: &str) -> Result<(), String> {
    let cap = value.to_ascii_uppercase();
    let cap = cap.as_str();
    if "ALL CHOWN DAC_OVERRIDE DAC_READ_SEARCH FOWNER FSETID KILL SETGID SETUID SETPCAP LINUX_IMMUTABLE NET_BIND_SERVICE NET_BROADCAST NET_ADMIN NET_RAW IPC_LOCK IPC_OWNER SYS_MODULE SYS_RAWIO SYS_CHROOT SYS_PTRACE SYS_PACCT SYS_ADMIN SYS_BOOT SYS_NICE SYS_RESOURCE SYS_TIME SYS_TTY_CONFIG MKNOD LEASE AUDIT_WRITE AUDIT_CONTROL SETFCAP MAC_OVERRIDE MAC_ADMIN SYSLOG WAKE_ALARM BLOCK_SUSPEND AUDIT_READ PERFMON BPF CHECKPOINT_RESTORE".split_whitespace().any(|s|s==cap){Ok(())}else{Err(format!("unknown capability: {value}"))}
}
fn sysctl(key: &str) -> Result<(), String> {
    if key.starts_with("net.")
        || key.starts_with("fs.mqueue.")
        || matches!(
            key,
            "kernel.shmmax"
                | "kernel.shmmni"
                | "kernel.shmall"
                | "kernel.shm_rmid_forced"
                | "kernel.msgmax"
                | "kernel.msgmni"
                | "kernel.msgmnb"
                | "kernel.sem"
        )
    {
        return Ok(());
    }
    if key == "kernel.pid_max"
        && std::fs::read_to_string("/proc/kallsyms")
            .is_ok_and(|s| s.contains("proc_dointvec_pidmax"))
    {
        return Ok(());
    }
    Err(format!("sysctl is not namespaced: {key}"))
}
fn security(value: &str) -> Result<String, String> {
    if value == "no-new-privileges" {
        return Ok(value.into());
    }
    let (k, v) = pair(value)?;
    match k {
        "seccomp" if v == "unconfined" => Ok(value.into()),
        "seccomp" => {
            let data = read_file(v)?;
            let config: Value =
                serde_json::from_str(&data).map_err(|e| format!("seccomp JSON: {e}"))?;
            Ok(format!("seccomp={config}"))
        }
        "label" => Ok(value.into()),
        _ => Err(format!("invalid security option: {value}")),
    }
}
fn ulimit(value: &str) -> Result<Value, String> {
    let (k, v) = pair(value)?;
    if !"as core cpu data fsize locks memlock msgqueue nice nofile nproc rss rtprio rttime sigpending stack".split_whitespace().any(|s|s==k){return Err("invalid ulimit name".into());}
    let (soft, hard) = v.split_once(':').unwrap_or((v, v));
    let soft = integer(soft)?;
    let hard = integer(hard)?;
    if soft < -1 || hard < -1 || (hard != -1 && (soft == -1 || soft > hard)) {
        return Err("invalid ulimit range".into());
    }
    let mut v = json!({"Name":k});
    set_number(&mut v, "Soft", soft);
    set_number(&mut v, "Hard", hard);
    Ok(v)
}
fn human_size(mut size: i64) -> String {
    for unit in ["B", "KB", "MB", "GB", "TB", "PB"] {
        if size < 1024 || size % 1024 != 0 {
            return format!("{size}{unit}");
        }
        size /= 1024;
    }
    format!("{size}EB")
}
fn host_channel(value: &str) -> Result<Value, String> {
    let p: Vec<_> = value.split(':').collect();
    if p.len() != 4 {
        return Err("host channel requires host:container:mode:size".into());
    }
    let src = absolute(p[0])?;
    let dst = absolute(p[1])?;
    if std::path::Path::new(&src).is_dir() {
        return Err("host channel directory already exists".into());
    }
    let perm = if p[2].is_empty() { "rw" } else { p[2] };
    if !mode(perm) {
        return Err("invalid host channel mode".into());
    }
    let size = if p[3].is_empty() {
        64 * 1024 * 1024
    } else {
        bytes(p[3])?
    };
    if size < 4096 {
        return Err("host channel size must be at least 4KB".into());
    }
    let mem = std::fs::read_to_string("/proc/meminfo").map_err(|e| e.to_string())?;
    for (key, divisor) in [("MemTotal:", 2), ("MemAvailable:", 1)] {
        let limit = mem
            .lines()
            .find(|s| s.starts_with(key))
            .and_then(|s| s.split_whitespace().nth(1))
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or("cannot read available memory")?
            * 1024
            / divisor;
        if size > limit {
            return Err("host channel size exceeds available memory".into());
        }
    }
    Ok(json!({"PathOnHost":src,"PathInContainer":dst,"Permissions":perm,"Size":size}))
}
fn log_option(value: &str, annotations: &mut Value) -> Result<(), String> {
    let (k, v) = pair(value)?;
    if v.is_empty() {
        return Err("empty log option".into());
    }
    let key = match k {
        "max-size" => "log.console.filesize",
        "max-file" => {
            if integer(v)
                .ok()
                .filter(|n| *n > 0 && *n <= u32::MAX as i64)
                .is_none()
            {
                return Err("invalid max-file".into());
            }
            "log.console.filerotate"
        }
        "disable-log" => match v {
            "false" => return Ok(()),
            "true" => {
                annotations["log.console.file"] = json!("none");
                return Ok(());
            }
            _ => return Err("invalid disable-log".into()),
        },
        "syslog-tag" => "log.console.tag",
        "syslog-facility" => {
            if !"kern user mail daemon auth syslog lpr news uucp cron authpriv ftp local0 local1 local2 local3 local4 local5 local6 local7".split_whitespace().any(|s|s==v){return Err("invalid syslog facility".into());}
            "log.console.facility"
        }
        _ => return Err(format!("unknown log option: {k}")),
    };
    annotations[key] = json!(v);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(args: &[&str]) -> Invocation {
        let mut all = vec!["create".to_string()];
        all.extend(args.iter().map(|s| s.to_string()));
        let crate::cli::Action::Invoke(c) = crate::cli::parse(&all, None).unwrap() else {
            panic!()
        };
        c
    }
    #[test]
    fn files_preserve_final_value_and_explicit_options_override() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("env");
        std::fs::write(&path, "DEMO=file\nTAIL=complete").unwrap();
        let (request, _) = build(&command(&[
            "--env-file",
            path.to_str().unwrap(),
            "--env=DEMO=explicit",
            "--env=PATH",
            "busybox",
        ]))
        .unwrap();
        let config: Value = serde_json::from_str(&request.customconfig).unwrap();
        assert_eq!(config["Env"][0], "DEMO=file");
        assert_eq!(config["Env"][1], "TAIL=complete");
        assert_eq!(config["Env"][2], "DEMO=explicit");
        assert_eq!(
            config["Env"][3],
            format!("PATH={}", std::env::var("PATH").unwrap())
        );
    }
    #[test]
    fn env_target_rejects_symlink_outside_rootfs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("rootfs");
        std::fs::create_dir(&root).unwrap();
        std::os::unix::fs::symlink(dir.path(), root.join("outside")).unwrap();
        let e = build(&command(&[
            "--external-rootfs",
            root.to_str().unwrap(),
            "--env-target-file=/outside/env",
            "none",
        ]))
        .unwrap_err();
        assert_eq!(e.exit_code, 125);
        assert!(e.message.contains("escapes"));
    }
    #[test]
    fn iops_accepts_unsigned_64_bit_rate() {
        let (r, _) = build(&command(&[
            "--device-read-iops=/dev/null:18446744073709551615",
            "busybox",
        ]))
        .unwrap();
        let host: Value = serde_json::from_str(&r.hostconfig).unwrap();
        assert_eq!(
            host["BlkioDeviceReadIops"][0]["Rate"].as_u64(),
            Some(u64::MAX)
        );
        assert!(build(&command(&[
            "--device-read-iops=/dev/null:18446744073709551616",
            "busybox"
        ]))
        .is_err());
    }
    #[test]
    fn requests_match_native_client() {
        let cases: Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/create-native.json"
        ))
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        for (name, contents) in [
            ("env", "A=from-file\n# comment\nEMPTY=\nLAST=with-newline\n"),
            ("label", "A=label\nFLAG\n# comment\nLAST=no-newline"),
            ("seccomp", r#"{"defaultAction":"SCMP_ACT_ALLOW"}"#),
            ("hooks", "{}"),
        ] {
            std::fs::write(dir.path().join(name), contents).unwrap();
        }
        for original in cases.as_array().unwrap() {
            let mut text = original.to_string();
            for name in ["env", "label", "seccomp", "hooks"] {
                text = text.replace(
                    &format!("/tmp/risula-create-{name}"),
                    dir.path().join(name).to_str().unwrap(),
                );
            }
            let case: Value = serde_json::from_str(&text).unwrap();
            let mut args = vec!["create".to_string()];
            args.extend(
                case["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string()),
            );
            let actual = crate::cli::parse(&args, None).and_then(|action| match action {
                crate::cli::Action::Invoke(command) => build(&command),
                _ => panic!(),
            });
            if case["code"] != 0 {
                assert_eq!(
                    actual.unwrap_err().exit_code,
                    case["code"].as_i64().unwrap() as i32,
                    "{args:?}"
                );
                continue;
            }
            let (r, _) = actual.unwrap_or_else(|e| panic!("{args:?}: {e:?}"));
            let actual = json!({"id":r.id,"image":r.image,"runtime":r.runtime,"rootfs":r.rootfs,"host":serde_json::from_str::<Value>(&r.hostconfig).unwrap(),"container":serde_json::from_str::<Value>(&r.customconfig).unwrap()});
            assert_eq!(actual, case["requests"][0], "{args:?}");
        }
    }
}
