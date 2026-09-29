use std::collections::HashMap;

use chrono::{Local, TimeZone};

use crate::cli::Invocation;
use crate::grpc::{self, images_proto::Image};

pub async fn run(command: &Invocation) -> Result<(), String> {
    let mut filters = HashMap::new();
    let mut quiet = false;
    for option in &command.options {
        match option.name {
            "quiet" => quiet = true,
            "filter" => {
                let text = option.value.as_deref().unwrap_or("");
                let (key, value) = text
                    .split_once('=')
                    .ok_or_else(|| format!("bad filter '{text}', expected name=value"))?;
                if key.trim().is_empty() {
                    return Err("filter name cannot be empty".to_string());
                }
                filters.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
            }
            _ => {}
        }
    }

    let mut images = grpc::list_images(&command.host, filters).await?;
    images.sort_by(|left, right| {
        let created = |image: &Image| {
            image
                .created_at
                .as_ref()
                .map(|time| (time.seconds, time.nanos))
                .unwrap_or_default()
        };
        created(right).cmp(&created(left))
    });

    if quiet {
        for image in &images {
            let id = short_digest(image);
            let id = if id == "-" { "<none>" } else { &id };
            println!("{id:<12}");
        }
    } else {
        print_table(&images);
    }
    Ok(())
}

fn short_digest(image: &Image) -> String {
    let digest = image
        .target
        .as_ref()
        .map(|target| target.digest.as_str())
        .unwrap_or("");
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return "-".to_string();
    };
    if hex.len() == 64
        && hex
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        hex[..12].to_string()
    } else {
        "-".to_string()
    }
}

fn repository_and_tag(name: &str) -> (&str, &str) {
    if name.is_empty() || name == "-" {
        return ("<none>", "<none>");
    }
    match name.rsplit_once(':') {
        Some((repository, tag)) if !tag.contains('/') => (repository, tag),
        _ => (name, "<none>"),
    }
}

fn created_at(image: &Image) -> String {
    let seconds = image
        .created_at
        .as_ref()
        .map(|time| time.seconds)
        .unwrap_or(0);
    let Some(time) = Local.timestamp_opt(seconds, 0).single() else {
        return "-".to_string();
    };
    time.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn size(image: &Image) -> String {
    let bytes = image.target.as_ref().map(|target| target.size).unwrap_or(0);
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.3}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.3}MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.3}KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes}B")
    }
}

fn print_row(values: [&str; 5], widths: &[usize; 5]) {
    for (value, width) in values.iter().zip(widths) {
        print!("{value:<width$} ", width = *width);
    }
    println!();
}

fn print_table(images: &[Image]) {
    let mut widths = [30, 10, 20, 20, 10];
    let rows: Vec<[String; 5]> = images
        .iter()
        .map(|image| {
            let (repository, tag) = repository_and_tag(&image.name);
            [
                repository.to_string(),
                tag.to_string(),
                short_digest(image),
                created_at(image),
                size(image),
            ]
        })
        .collect();
    for row in &rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }

    print_row(
        ["REPOSITORY", "TAG", "IMAGE ID", "CREATED", "SIZE"],
        &widths,
    );
    for row in &rows {
        print_row([&row[0], &row[1], &row[2], &row[3], &row[4]], &widths);
    }
}
