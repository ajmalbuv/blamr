use crate::stats::Stats;
use std::borrow::Cow;
use std::env;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const AUTHOR_MIN_WIDTH: usize = 25;
// CJK characters are double-width (2 columns each), so a 40-column cap may
// render fewer visible characters for non-Latin author names. Intentional:
// it keeps the table from overflowing typical terminal widths.
const AUTHOR_MAX_WIDTH: usize = 40;
const LINES_WIDTH: usize = 8;
const PERCENT_WIDTH: usize = 8;
const BAR_MAX: usize = 20;
const PROGRESS_PATH_WIDTH: usize = 60;
const PROGRESS_MIN_PATH_WIDTH: usize = 20;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Copy)]
pub struct TableOptions {
    pub ascii: bool,
    pub by_email: bool,
}

impl TableOptions {
    fn divider(self) -> char {
        if self.ascii { '-' } else { '─' }
    }

    fn bar(self) -> char {
        if self.ascii { '#' } else { '█' }
    }
}

pub fn sanitize_terminal(value: &str) -> Cow<'_, str> {
    if value.chars().any(char::is_control) {
        Cow::Owned(
            value
                .chars()
                .map(|character| {
                    if character.is_control() {
                        '?'
                    } else {
                        character
                    }
                })
                .collect(),
        )
    } else {
        Cow::Borrowed(value)
    }
}

pub fn terminal_width() -> usize {
    env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|width| *width >= 40)
        .or_else(console_width)
        .unwrap_or(80)
}

#[cfg(windows)]
fn console_width() -> Option<usize> {
    use windows_sys::Win32::System::Console::{
        CONSOLE_SCREEN_BUFFER_INFO, GetConsoleScreenBufferInfo, GetStdHandle, STD_ERROR_HANDLE,
    };

    // SAFETY: the handle is not owned, and the output pointer is valid for writes.
    unsafe {
        let handle = GetStdHandle(STD_ERROR_HANDLE);
        let mut info = std::mem::zeroed::<CONSOLE_SCREEN_BUFFER_INFO>();
        if GetConsoleScreenBufferInfo(handle, &mut info) == 0 {
            return None;
        }
        let width = (info.srWindow.Right - info.srWindow.Left + 1) as usize;
        (width >= 40).then_some(width)
    }
}

#[cfg(not(windows))]
fn console_width() -> Option<usize> {
    None
}

pub fn unicode_supported(is_tty: bool) -> bool {
    if !is_tty {
        return true;
    }

    if env::var_os("TERM").is_some_and(|term| term == "dumb") {
        return false;
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::GetConsoleOutputCP;
        // SAFETY: GetConsoleOutputCP has no preconditions and does not write memory.
        unsafe { GetConsoleOutputCP() == 65001 }
    }

    #[cfg(not(windows))]
    {
        true
    }
}

pub fn truncate_width(value: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut width = 0;

    for character in value.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if width + character_width > max_width {
            break;
        }
        out.push(character);
        width += character_width;
    }

    out
}

pub struct Progress {
    enabled: bool,
    active: bool,
    terminal_width: usize,
    rendered_width: usize,
    last_draw: Option<Instant>,
}

impl Progress {
    pub fn new(enabled: bool, terminal_width: usize) -> Self {
        Self {
            enabled,
            active: false,
            terminal_width,
            rendered_width: 0,
            last_draw: None,
        }
    }

    pub fn update(
        &mut self,
        out: &mut dyn Write,
        done: usize,
        total: usize,
        file: &str,
    ) -> io::Result<()> {
        if !self.enabled || total == 0 {
            return Ok(());
        }

        if done < total
            && self
                .last_draw
                .is_some_and(|last| last.elapsed() < PROGRESS_INTERVAL)
        {
            return Ok(());
        }

        let line = progress_line(done, total, file, self.terminal_width);
        self.rendered_width = line.width();
        self.last_draw = Some(Instant::now());
        self.active = true;

        write!(out, "\r{line}")?;
        out.flush()
    }

    pub fn clear(&mut self, out: &mut dyn Write) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        write!(out, "\r{}\r", " ".repeat(self.rendered_width.max(1)))?;
        out.flush()?;
        self.active = false;
        self.last_draw = None;
        Ok(())
    }

    pub fn finish(&mut self, out: &mut dyn Write) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        writeln!(out)?;
        out.flush()?;
        self.active = false;
        Ok(())
    }
}

fn progress_line(done: usize, total: usize, file: &str, terminal_width: usize) -> String {
    let digits = total.to_string().len();
    let prefix = format!("[{done:>digits$}/{total}] ");
    let path_width = terminal_width
        .saturating_sub(prefix.width() + 1)
        .clamp(PROGRESS_MIN_PATH_WIDTH, PROGRESS_PATH_WIDTH);
    let file = sanitize_terminal(file);
    let file = truncate_width(&file, path_width);

    let mut line = String::with_capacity(prefix.len() + path_width);
    line.push_str(&prefix);
    line.push_str(&file);
    line.push_str(&" ".repeat(path_width.saturating_sub(file.width())));
    line
}

pub fn print_table(out: &mut dyn Write, stats: &Stats, options: TableOptions) -> io::Result<()> {
    let author_header = if options.by_email { "Email" } else { "Author" };
    let author_width = stats
        .authors
        .iter()
        .map(|author| sanitize_terminal(&author.author).width())
        .chain(std::iter::once(author_header.width()))
        .max()
        .unwrap_or(AUTHOR_MIN_WIDTH)
        .clamp(AUTHOR_MIN_WIDTH, AUTHOR_MAX_WIDTH);
    let divider = options
        .divider()
        .to_string()
        .repeat(author_width + 1 + LINES_WIDTH + 1 + PERCENT_WIDTH + 2);

    let mut text = String::new();
    text.push_str(&divider);
    text.push('\n');
    text.push_str(&pad_right(author_header, author_width));
    text.push(' ');
    text.push_str(&pad_left("Lines", LINES_WIDTH));
    text.push(' ');
    text.push_str(&pad_left("%", PERCENT_WIDTH));
    text.push('\n');
    text.push_str(&divider);
    text.push('\n');

    for author in &stats.authors {
        let name = sanitize_terminal(&author.author);
        let name = truncate_width(&name, author_width);
        let bar_count = ((author.percentage / 5.0).floor() as usize).min(BAR_MAX);

        text.push_str(&pad_right(&name, author_width));
        text.push(' ');
        text.push_str(&pad_left(&author.lines.to_string(), LINES_WIDTH));
        text.push(' ');
        text.push_str(&pad_left(
            &format!("{:.1}%", author.percentage),
            PERCENT_WIDTH,
        ));
        text.push_str("  ");
        text.push_str(&options.bar().to_string().repeat(bar_count));
        text.push('\n');
    }

    text.push_str(&divider);
    text.push('\n');
    text.push_str(&pad_right("Total", author_width));
    text.push(' ');
    text.push_str(&pad_left(&stats.total_lines.to_string(), LINES_WIDTH));
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
    let length = value.width();
    if length >= width {
        value.to_owned()
    } else {
        format!("{value}{}", " ".repeat(width - length))
    }
}

fn pad_left(value: &str, width: usize) -> String {
    let length = value.width();
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
    fn truncates_by_display_width() {
        assert_eq!(truncate_width("hello", 10), "hello");
        assert_eq!(truncate_width("hello world", 5), "hello");
        assert_eq!(truncate_width("🚀🚀🚀", 2), "🚀");
        assert_eq!(truncate_width("🚀🚀🚀", 4), "🚀🚀");
    }

    #[test]
    fn sanitizes_control_characters() {
        assert_eq!(sanitize_terminal("Alice"), "Alice");
        assert_eq!(sanitize_terminal("Ali\x1b[2Jce\n"), "Ali?[2Jce?");
    }

    #[test]
    fn progress_avoids_ansi_and_respects_terminal_state() {
        let mut buffer = Vec::new();
        let mut progress = Progress::new(false, 80);
        progress.update(&mut buffer, 1, 10, "main.rs").unwrap();
        assert!(buffer.is_empty());

        let mut progress = Progress::new(true, 80);
        progress.update(&mut buffer, 1, 10, "main.rs").unwrap();
        let output = String::from_utf8(buffer.clone()).unwrap();
        assert!(output.starts_with("\r[ 1/10]"));
        assert!(!output.contains('\x1b'));
        assert!(output.contains("main.rs"));

        progress.clear(&mut buffer).unwrap();
        progress.finish(&mut buffer).unwrap();
    }

    #[test]
    fn progress_line_fits_and_has_stable_width() {
        let narrow = progress_line(1, 10, &"x".repeat(200), 40);
        assert!(narrow.width() <= 40);

        let first = progress_line(1, 10, "main.rs", 80);
        let last = progress_line(9, 10, "main.rs", 80);
        assert_eq!(first.width(), last.width());
    }

    #[test]
    fn ascii_table_uses_only_ascii_glyphs() {
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
        print_table(
            &mut buffer,
            &stats,
            TableOptions {
                ascii: true,
                by_email: false,
            },
        )
        .unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.is_ascii());
        assert!(output.contains('#'));
        assert!(!output.contains('─'));
        assert!(!output.contains('█'));
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
        print_table(
            &mut buffer,
            &stats,
            TableOptions {
                ascii: false,
                by_email: false,
            },
        )
        .unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(!output.starts_with('\n'));
        assert!(output.contains("Author"));
        assert!(output.contains("Lines"));
        assert!(output.contains("Alice"));
        assert!(output.contains("70.0%"));
        assert!(output.contains("Bob"));
        assert!(output.contains("30.0%"));
        assert!(output.contains("Total"));
        assert!(output.contains("100"));
        assert!(output.contains('█'));
    }

    #[test]
    fn email_header_is_used_when_requested() {
        let stats = Stats {
            total_files: 1,
            total_lines: 1,
            authors: vec![AuthorStat {
                author: "alice@example.com".to_owned(),
                lines: 1,
                percentage: 100.0,
            }],
        };

        let mut buffer = Vec::new();
        print_table(
            &mut buffer,
            &stats,
            TableOptions {
                ascii: false,
                by_email: true,
            },
        )
        .unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("Email"));
        assert!(!output.contains("Author"));
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
