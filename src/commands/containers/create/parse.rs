use serde_json::{json, Value};

pub(crate) fn integer(value: &str) -> Result<i64, String> {
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (sign, text) = if let Some(s) = value.strip_prefix('-') {
        (-1_i128, s)
    } else {
        (1, value.strip_prefix('+').unwrap_or(value))
    };
    let (base, text) = if let Some(s) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))
    {
        (16, s)
    } else if text.starts_with('0') {
        (8, text)
    } else {
        (10, text)
    };
    if text.is_empty() || !text.chars().all(|c| c.is_digit(base)) {
        return Err(format!("invalid integer: {value}"));
    }
    i128::from_str_radix(text, base)
        .ok()
        .and_then(|n| i64::try_from(n * sign).ok())
        .ok_or_else(|| format!("integer out of range: {value}"))
}

pub(super) fn unsigned(value: &str) -> Result<u64, String> {
    let text = value.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let text = text.strip_prefix('+').unwrap_or(text);
    let (radix, text) =
        if let Some(rest) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            (16, rest)
        } else if text.starts_with('0') {
            (8, text)
        } else {
            (10, text)
        };
    if text.is_empty() || !text.chars().all(|c| c.is_digit(radix)) {
        return Err(format!("invalid unsigned integer: {value}"));
    }
    u64::from_str_radix(text, radix).map_err(|_| format!("unsigned integer out of range: {value}"))
}

// Keep upstream base-0 integer parsing and scale the fraction without float rounding.
pub(crate) fn scaled(value: &str, scale: i64) -> Result<i64, String> {
    let (negative, text) = if let Some(s) = value.strip_prefix('-') {
        (true, s)
    } else {
        (false, value.strip_prefix('+').unwrap_or(value))
    };
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if (text.contains('.') && fraction.is_empty())
        || whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || !fraction.bytes().all(|c| c.is_ascii_digit())
        || fraction.len() > 18
    {
        return Err(format!("invalid quantity: {value}"));
    }
    let whole = integer(whole)? as i128;
    let fraction: i128 = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse()
            .map_err(|_| format!("invalid fraction: {value}"))?
    };
    let denominator = 10_i128.pow(text.split_once('.').map_or(0, |(_, f)| f.len()) as u32);
    let n = whole
        .checked_mul(scale as i128)
        .and_then(|n| n.checked_add(fraction * scale as i128 / denominator))
        .ok_or("quantity out of range")?;
    i64::try_from(if negative { -n } else { n }).map_err(|_| "quantity out of range".into())
}

pub(crate) fn bytes(value: &str) -> Result<i64, String> {
    if !value.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        return Err(format!("invalid byte size: {value}"));
    }
    let end = value
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(value.len());
    let suffix = value[end..].to_ascii_lowercase();
    let power = match suffix.as_str() {
        "" | "b" | "i" | "ib" => 0,
        "k" | "kb" | "ki" | "kib" => 1,
        "m" | "mb" | "mi" | "mib" => 2,
        "g" | "gb" | "gi" | "gib" => 3,
        "t" | "tb" | "ti" | "tib" => 4,
        "p" | "pb" | "pi" | "pib" => 5,
        _ => return Err(format!("invalid byte unit: {value}")),
    };
    scaled(&value[..end], 1024_i64.pow(power))
}

pub(super) fn duration(value: &str) -> Result<i64, String> {
    // iSula health durations accept a single quantity and unit.
    for (suffix, scale) in [
        ("ms", 1_000_000),
        ("s", 1_000_000_000),
        ("m", 60_000_000_000),
        ("h", 3_600_000_000_000),
    ] {
        if let Some(n) = value.strip_suffix(suffix) {
            if n.is_empty() || !n.bytes().all(|c| c.is_ascii_digit()) {
                return Err(format!("invalid duration: {value}"));
            }
            let n = integer(n)?
                .checked_mul(scale)
                .ok_or("duration out of range")?;
            if n >= 0 {
                return Ok(n);
            }
        }
    }
    Err(format!("invalid health duration: {value}"))
}

pub(super) fn pair(value: &str) -> Result<(&str, &str), String> {
    value
        .split_once('=')
        .filter(|(k, _)| !k.is_empty())
        .ok_or_else(|| format!("expected key=value: {value}"))
}

pub(super) fn name(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}

pub(super) fn absolute(value: &str) -> Result<String, String> {
    if !value.starts_with('/') || value.len() >= 4096 {
        return Err(format!("expected an absolute path: {value}"));
    }
    let mut parts = Vec::new();
    for p in value.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(p),
        }
    }
    Ok(format!("/{}", parts.join("/")))
}

pub(super) fn mode(value: &str) -> bool {
    let mut groups = std::collections::HashSet::new();
    value.split(',').all(|m| {
        let g = match m {
            "ro" | "rw" => 0,
            "z" | "Z" => 1,
            "private" | "rprivate" | "shared" | "rshared" | "slave" | "rslave" => 2,
            "nocopy" => 3,
            _ => return false,
        };
        groups.insert(g)
    })
}

pub(super) fn volume(value: &str) -> Result<(), String> {
    let p: Vec<_> = value.split(':').collect();
    let target = match p.as_slice() {
        [target] if !target.is_empty() => return Ok(()),
        [source, target] if !source.is_empty() && !mode(target) => *target,
        [source, target, m] if !source.is_empty() && mode(m) => *target,
        _ => return Err(format!("invalid volume: {value}")),
    };
    if absolute(target)? == "/" {
        return Err("volume destination cannot be /".into());
    }
    Ok(())
}

pub(super) fn mount(value: &str) -> Result<Value, String> {
    let mut m = json!({});
    for item in value.split(',') {
        let (k, v) = item.split_once('=').unwrap_or((item, ""));
        let key = match k {
            "type" => "Type",
            "src" | "source" => "Source",
            "dst" | "destination" | "target" => "Target",
            "ro" | "readonly" => "Readonly",
            "bind-propagation" => "BindOptions",
            "bind-selinux-opts" => "BindOptions",
            "volume-nocopy" => "VolumeOptions",
            "tmpfs-size" | "tmpfs-mode" => "TmpfsOptions",
            _ => return Err(format!("unknown mount option: {k}")),
        };
        match k {
            "ro" | "readonly" => {
                m[key] = json!(if v.is_empty() {
                    true
                } else {
                    crate::cli::parse_bool(Some(v), k)?
                });
            }
            "volume-nocopy" => {
                if !v.is_empty() {
                    m[key] = json!({});
                    if crate::cli::parse_bool(Some(v), k)? {
                        m[key]["NoCopy"] = json!(true);
                    }
                }
            }
            "bind-propagation" | "bind-selinux-opts" => {
                if (k == "bind-propagation"
                    && !matches!(
                        v,
                        "private" | "rprivate" | "shared" | "rshared" | "slave" | "rslave"
                    ))
                    || (k == "bind-selinux-opts" && !matches!(v, "z" | "Z"))
                {
                    return Err(format!("invalid mount mode: {v}"));
                }
                if m.get(key).is_none() {
                    m[key] = json!({});
                }
                m[key][if k == "bind-propagation" {
                    "Propagation"
                } else {
                    "SelinuxOpts"
                }] = json!(v);
            }
            "tmpfs-size" | "tmpfs-mode" => {
                if m.get(key).is_none() {
                    m[key] = json!({});
                }
                let n = if k == "tmpfs-size" {
                    bytes(v)?
                } else {
                    i64::from_str_radix(v, 8).map_err(|_| "invalid tmpfs mode")?
                };
                if n < 0 {
                    return Err("negative tmpfs option".into());
                }
                m[key][if k == "tmpfs-size" {
                    "SizeBytes"
                } else {
                    "Mode"
                }] = json!(n);
            }
            _ => {
                if m.get(key).is_some() {
                    return Err(format!("duplicate mount field: {k}"));
                }
                m[key] = json!(v);
            }
        }
    }
    let kind = m["Type"].as_str().unwrap_or("volume").to_string();
    if !matches!(kind.as_str(), "bind" | "volume" | "tmpfs" | "squashfs") {
        return Err("invalid mount type".into());
    }
    m["Type"] = json!(kind);
    let target = absolute(m["Target"].as_str().ok_or("mount requires a target")?)?;
    if target == "/" {
        return Err("mount target cannot be /".into());
    }
    m["Target"] = json!(target);
    if matches!(kind.as_str(), "bind" | "squashfs") {
        m["Source"] = json!(absolute(
            m["Source"].as_str().ok_or("mount requires a source")?
        )?);
        if m.get("VolumeOptions").is_some() {
            return Err("volume options require a volume mount".into());
        }
    }
    if kind == "volume"
        && m.get("Source").is_some_and(|s| {
            let s = s.as_str().unwrap_or("");
            s.len() < 2 || s.len() > 64 || !name(s)
        })
    {
        return Err("invalid volume name".into());
    }
    if kind == "volume"
        && (m.get("BindOptions").is_some()
            || (m.get("Readonly").is_some() && m.get("Source").is_none()))
    {
        return Err("invalid anonymous volume options".into());
    }
    if kind == "tmpfs"
        && ["Source", "BindOptions", "VolumeOptions"]
            .iter()
            .any(|k| m.get(k).is_some())
    {
        return Err("invalid tmpfs mount options".into());
    }
    if kind != "tmpfs" && m.get("TmpfsOptions").is_some() {
        return Err("tmpfs options require a tmpfs mount".into());
    }
    if kind == "squashfs" {
        std::fs::metadata(m["Source"].as_str().unwrap()).map_err(|e| e.to_string())?;
    }
    if m["Readonly"] == false {
        m.as_object_mut().unwrap().remove("Readonly");
    }
    Ok(m)
}

pub(super) fn device(value: &str) -> Result<Value, String> {
    let p: Vec<_> = value.split(':').collect();
    let (src, dst, perm) = match p.as_slice() {
        [src] => (*src, *src, "rwm"),
        [src, p] if !p.starts_with('/') => (*src, *src, *p),
        [src, dst] => (*src, *dst, "rwm"),
        [src, dst, p] => (*src, *dst, *p),
        _ => return Err("invalid device format".into()),
    };
    absolute(src)?;
    absolute(dst)?;
    if !dst.starts_with("/dev/") || perm.is_empty() || !perm.bytes().all(|c| b"rwm".contains(&c)) {
        return Err("invalid device path or permissions".into());
    }
    Ok(json!({"PathOnHost":src,"PathInContainer":dst,"CgroupPermissions":perm}))
}

pub(super) fn ports(value: &str) -> Result<Vec<u16>, String> {
    let (start, end) = value.split_once('-').unwrap_or((value, value));
    let start: u16 = start.parse().map_err(|_| "invalid port")?;
    let end: u16 = end.parse().map_err(|_| "invalid port")?;
    if start == 0 || start > end {
        return Err("invalid port range".into());
    }
    Ok((start..=end).collect())
}

pub(super) fn publish(value: &str, expose: &mut Value, bindings: &mut Value) -> Result<(), String> {
    let (text, proto) = value.split_once('/').unwrap_or((value, "tcp"));
    if !matches!(proto, "tcp" | "udp" | "sctp") {
        return Err("invalid port protocol".into());
    }
    let parts: Vec<_> = text.rsplitn(3, ':').collect();
    let container = ports(parts[0])?;
    let host = if parts.get(1).is_some_and(|s| !s.is_empty()) {
        ports(parts[1])?
    } else {
        Vec::new()
    };
    if !host.is_empty() && host.len() != container.len() {
        return Err("port ranges have different lengths".into());
    }
    let ip = if host.is_empty() {
        ""
    } else {
        parts.get(2).copied().unwrap_or("")
    };
    if !ip.is_empty() {
        ip.parse::<std::net::Ipv4Addr>()
            .map_err(|_| "invalid publish IP")?;
    }
    for (i, port) in container.iter().enumerate() {
        let key = format!("{port}/{proto}");
        expose[&key] = json!({});
        let mut entry = json!({});
        if !ip.is_empty() {
            entry["HostIP"] = json!(ip);
        }
        if let Some(p) = host.get(i) {
            entry["HostPort"] = json!(p.to_string());
        }
        if bindings.get(&key).is_none() {
            bindings[&key] = json!({"Element":{"host":[]}});
        }
        bindings[&key]["Element"]["host"]
            .as_array_mut()
            .unwrap()
            .push(entry);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quantities_check_overflow_and_keep_integer_precision() {
        assert_eq!(bytes("1.5G").unwrap(), 1_610_612_736);
        assert_eq!(bytes("1GiB").unwrap(), 1_073_741_824);
        assert_eq!(bytes("010m").unwrap(), 8 * 1024 * 1024);
        assert!(bytes("08m").is_err());
        assert_eq!(bytes("9223372036854775807").unwrap(), i64::MAX);
        for value in [
            "9223372036854775808",
            "99999999999999999999999999999999999999999999",
            "-1",
            "+1",
            "NaN",
            "1XB",
        ] {
            assert!(bytes(value).is_err(), "{value}");
        }
        assert_eq!(scaled("1.123456789", 1_000_000_000).unwrap(), 1_123_456_789);
        assert_eq!(duration("500ms").unwrap(), 500_000_000);
        assert_eq!(duration("010s").unwrap(), 8_000_000_000);
        for value in ["1.5s", "-1s", "1m30s", "08s", "9223372036854775807h"] {
            assert!(duration(value).is_err(), "{value}");
        }
    }
}
