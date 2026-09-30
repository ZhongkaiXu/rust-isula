use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt::Write;

use crate::cli::Invocation;
use crate::grpc::{self, images_proto::SearchImage};

const DEFAULT_FORMAT: &str =
    "table {{.Name}}\t{{.Description}}\t{{.StarCount}}\t{{.IsOfficial}}\t{{.IsAutomated}}";

#[derive(Clone, Copy)]
enum Field {
    Name,
    Description,
    StarCount,
    IsOfficial,
    IsAutomated,
}

impl Field {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "Name" => Some(Self::Name),
            "Description" => Some(Self::Description),
            "StarCount" => Some(Self::StarCount),
            "IsOfficial" => Some(Self::IsOfficial),
            "IsAutomated" => Some(Self::IsAutomated),
            _ => None,
        }
    }

    fn column(self) -> usize {
        match self {
            Self::Name => 0,
            Self::Description => 1,
            Self::StarCount => 2,
            Self::IsOfficial => 3,
            Self::IsAutomated => 4,
        }
    }

    fn header(self) -> &'static str {
        match self {
            Self::Name => "NAME",
            Self::Description => "DESCRIPTION",
            Self::StarCount => "STARS",
            Self::IsOfficial => "OFFICIAL",
            Self::IsAutomated => "AUTOMATED",
        }
    }

    fn value(self, image: &SearchImage, no_trunc: bool) -> String {
        match self {
            Self::Name => present(&image.name).to_string(),
            Self::Description => {
                let description = present(&image.description);
                if !no_trunc && description.chars().count() > 48 {
                    format!("{}...", description.chars().take(44).collect::<String>())
                } else {
                    description.to_string()
                }
            }
            Self::StarCount => image.star_count.to_string(),
            Self::IsOfficial => if image.is_official { "[OK]" } else { " " }.to_string(),
            Self::IsAutomated => if image.is_automated { "[OK]" } else { " " }.to_string(),
        }
    }
}

fn present(text: &str) -> &str {
    if text.is_empty() {
        "-"
    } else {
        text
    }
}

enum Part {
    Text(String),
    Field(Field),
}

struct Format {
    table: bool,
    parts: Vec<Part>,
}

fn unescape(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some(next) => {
                    result.push('\\');
                    result.push(next);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn parse_format(source: &str) -> Result<Format, String> {
    let source = if source == "table" {
        DEFAULT_FORMAT
    } else {
        source
    };
    let (table, mut remaining) = match source.strip_prefix("table ") {
        Some(rest) => (true, rest),
        None => (false, source),
    };
    let mut parts = Vec::new();
    let mut fields = 0;

    while let Some(start) = remaining.find("{{") {
        let literal = &remaining[..start];
        if literal.contains("}}") {
            return Err("invalid search format".to_string());
        }
        if !(table && parts.is_empty() && literal.trim().is_empty()) {
            parts.push(Part::Text(unescape(literal)));
        }
        let rest = &remaining[start + 2..];
        let end = rest.find("}}").ok_or("invalid search format")?;
        let name = rest[..end]
            .trim()
            .strip_prefix('.')
            .ok_or("invalid search format")?;
        let field =
            Field::parse(name).ok_or_else(|| format!("unsupported search field: {name}"))?;
        parts.push(Part::Field(field));
        fields += 1;
        remaining = &rest[end + 2..];
    }
    if remaining.contains("}}") || fields == 0 {
        return Err("invalid search format".to_string());
    }
    parts.push(Part::Text(unescape(remaining)));
    Ok(Format { table, parts })
}

fn widths(images: &[SearchImage], no_trunc: bool) -> [usize; 5] {
    let mut widths = [24, 48, 6, 9, 10];
    for image in images {
        widths[0] = widths[0].max(present(&image.name).chars().count());
        if no_trunc {
            widths[1] = widths[1].max(present(&image.description).chars().count());
        }
        widths[2] = widths[2].max(image.star_count.to_string().len());
    }
    widths
}

fn render(format: &Format, image: Option<&SearchImage>, widths: &[usize; 5], no_trunc: bool) {
    let mut line = String::new();
    for part in &format.parts {
        match part {
            Part::Text(text) => line.push_str(text),
            Part::Field(field) => {
                let value = match image {
                    Some(image) => field.value(image, no_trunc),
                    None => field.header().to_string(),
                };
                write!(line, "{value:<width$}    ", width = widths[field.column()]).unwrap();
            }
        }
    }
    println!("{line}");
}

pub async fn run(command: &Invocation) -> Result<(), String> {
    let mut limit = 25;
    let mut filters = HashMap::new();
    let mut no_trunc = false;
    let mut format_text = DEFAULT_FORMAT;

    for option in &command.options {
        match option.name {
            "limit" => {
                let value = option.value.as_deref().unwrap_or("");
                limit = value
                    .parse::<u32>()
                    .map_err(|_| "limit must be between 1 and 100")?;
                if limit == 0 {
                    limit = 25;
                }
                if limit > 100 {
                    return Err("limit must be between 1 and 100".to_string());
                }
            }
            "filter" => {
                let value = option.value.as_deref().unwrap_or("");
                let (key, value) = value
                    .split_once('=')
                    .ok_or_else(|| format!("bad filter '{value}', expected name=value"))?;
                let key = key.trim().to_ascii_lowercase();
                if key.is_empty() {
                    return Err("filter name cannot be empty".to_string());
                }
                filters.insert(key, value.trim().to_string());
            }
            "no-trunc" => no_trunc = true,
            "format" => format_text = option.value.as_deref().unwrap_or(""),
            _ => {}
        }
    }

    let format = parse_format(format_text)?;
    let mut images = grpc::search_images(&command.host, &command.args[0], limit, filters).await?;
    images.sort_by_key(|image| Reverse(image.star_count));
    let widths = widths(&images, no_trunc);
    if format.table {
        render(&format, None, &widths, no_trunc);
    }
    for image in &images {
        render(&format, Some(image), &widths, no_trunc);
    }
    Ok(())
}
