use crate::stats::Stats;
use std::io::{self, Write};

pub fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub fn print_progress(
    out: &mut dyn Write,
    done: usize,
    total: usize,
    current_file: &str,
    is_tty: bool,
) -> io::Result<()> {
    if !is_tty {
        return Ok(());
    }

    let file = truncate(current_file, 60);
    let file = pad_right(&file, 60);
    write!(out, "\r[{done}/{total}] {file}\x1b[K")?;
    out.flush()
}

pub fn print_table(out: &mut dyn Write, stats: &Stats) -> io::Result<()> {
    let divider = "─".repeat(45);
    let mut text = String::new();

    text.push_str("\n\n");
    text.push_str(&divider);
    text.push('\n');
    text.push_str(&pad_right("Author", 25));
    text.push(' ');
    text.push_str(&pad_left("Lines", 8));
    text.push(' ');
    text.push_str(&pad_left("%", 8));
    text.push('\n');
    text.push_str(&divider);
    text.push('\n');

    for author in &stats.authors {
        let bar_count = (author.percentage / 5.0).floor().max(0.0) as usize;
        text.push_str(&pad_right(&truncate(&author.author, 25), 25));
        text.push(' ');
        text.push_str(&pad_left(&author.lines.to_string(), 8));
        text.push(' ');
        text.push_str(&pad_left(&format!("{:.1}", author.percentage), 7));
        text.push_str("%  ");
        text.push_str(&"█".repeat(bar_count));
        text.push('\n');
    }

    text.push_str(&divider);
    text.push('\n');
    text.push_str(&pad_right("Total", 25));
    text.push(' ');
    text.push_str(&pad_left(&stats.total_lines.to_string(), 8));
    text.push('\n');

    out.write_all(text.as_bytes())
}

pub fn print_json(out: &mut dyn Write, stats: &Stats) -> io::Result<()> {
    let json = serde_json::to_string_pretty(stats)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    writeln!(out, "{json}")
}

pub fn version_string() -> String {
    let version = env!("CARGO_PKG_VERSION");

    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };

    format!("{version} ({os}/{arch})")
}

fn pad_right(value: &str, width: usize) -> String {
    let length = value.chars().count();
    if length >= width {
        value.to_owned()
    } else {
        format!("{value}{}", " ".repeat(width - length))
    }
}

fn pad_left(value: &str, width: usize) -> String {
    let length = value.chars().count();
    if length >= width {
        value.to_owned()
    } else {
        format!("{}{value}", " ".repeat(width - length))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::AuthorStat;

    #[test]
    fn truncates_by_character() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello");
        assert_eq!(truncate("🚀🚀🚀🚀🚀", 2), "🚀🚀");
    }

    #[test]
    fn progress_respects_terminal_state() {
        let mut buffer = Vec::new();
        print_progress(&mut buffer, 1, 10, "main.rs", false).unwrap();
        assert!(buffer.is_empty());

        print_progress(&mut buffer, 1, 10, "main.rs", true).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.starts_with("\r[1/10]"));
        assert!(output.ends_with("\x1b[K"));
    }

    #[test]
    fn table_contains_expected_rows() {
        let stats = Stats {
            total_files: 2,
            total_lines: 100,
            authors: vec![
                AuthorStat {
                    author: "Alice".to_owned(),
                    lines: 70,
                    percentage: 70.0,
                },
                AuthorStat {
                    author: "Bob".to_owned(),
                    lines: 30,
                    percentage: 30.0,
                },
            ],
        };

        let mut buffer = Vec::new();
        print_table(&mut buffer, &stats).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.starts_with("\n\n"));
        assert!(output.contains("Author"));
        assert!(output.contains("Lines"));
        assert!(output.contains("Alice"));
        assert!(output.contains("70"));
        assert!(output.contains("Bob"));
        assert!(output.contains("30"));
        assert!(output.contains("Total"));
        assert!(output.contains("100"));
        assert!(output.contains("██████████████"));
    }

    #[test]
    fn json_has_expected_shape() {
        let stats = Stats {
            total_files: 2,
            total_lines: 100,
            authors: vec![AuthorStat {
                author: "Alice".to_owned(),
                lines: 100,
                percentage: 100.0,
            }],
        };

        let mut buffer = Vec::new();
        print_json(&mut buffer, &stats).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert_eq!(value["total_files"], 2);
        assert_eq!(value["total_lines"], 100);
        assert_eq!(value["authors"][0]["author"], "Alice");
    }

    #[test]
    fn version_has_platform() {
        let version = version_string();
        assert!(version.contains('/'));
        assert!(!version.starts_with(" ("));
    }
}
