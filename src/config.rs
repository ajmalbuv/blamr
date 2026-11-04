use lexopt::prelude::*;
use std::ffi::OsString;
use std::io::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub extensions: Vec<String>,
    pub workers: usize,
    pub json_output: bool,
    pub show_version: bool,
    pub by_email: bool,
    pub ignore_blank: bool,
    pub committed_only: bool,
    pub ascii: bool,
    pub no_progress: bool,
    pub paths: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Help,
    Message(String),
}

pub fn parse(
    args: &[OsString],
    default_workers: usize,
    out: &mut dyn Write,
) -> Result<Config, ParseError> {
    let default_workers = default_workers.max(1);
    let normalized = normalize_args(args);
    let mut parser = lexopt::Parser::from_args(normalized);

    let mut extensions_raw: Option<String> = None;
    let mut workers_raw: Option<i64> = None;
    let mut version_flag = false;
    let mut v_flag = false;
    let mut json_output = false;
    let mut by_email = false;
    let mut ignore_blank = false;
    let mut committed_only = false;
    let mut ascii = false;
    let mut no_progress = false;
    let mut paths = Vec::new();

    while let Some(arg) = parser
        .next()
        .map_err(|e| fail(out, default_workers, e.to_string()))?
    {
        match arg {
            Short('h') | Long("help") => {
                return Err(ParseError::Help);
            }
            Short('v') => {
                v_flag = true;
            }
            Long("v") => {
                v_flag = parse_bool_opt(parser.optional_value(), "v", out, default_workers)?;
            }
            Long("version") => {
                version_flag =
                    parse_bool_opt(parser.optional_value(), "version", out, default_workers)?;
            }
            Long("json") => {
                json_output =
                    parse_bool_opt(parser.optional_value(), "json", out, default_workers)?;
            }
            Short('e') => {
                by_email = true;
            }
            Long("email") => {
                by_email = parse_bool_opt(parser.optional_value(), "email", out, default_workers)?;
            }
            Short('w') => {
                ignore_blank = true;
            }
            Long("ignore-blank") => {
                ignore_blank = parse_bool_opt(
                    parser.optional_value(),
                    "ignore-blank",
                    out,
                    default_workers,
                )?;
            }
            Short('c') => {
                committed_only = true;
            }
            Long("committed-only") => {
                committed_only = parse_bool_opt(
                    parser.optional_value(),
                    "committed-only",
                    out,
                    default_workers,
                )?;
            }
            Short('a') => {
                ascii = true;
            }
            Long("ascii") => {
                ascii = parse_bool_opt(parser.optional_value(), "ascii", out, default_workers)?;
            }
            Short('n') => {
                no_progress = true;
            }
            Long("no-progress") => {
                no_progress =
                    parse_bool_opt(parser.optional_value(), "no-progress", out, default_workers)?;
            }
            Short('x') | Long("ext") | Long("extension") => {
                let val = parser.value().map_err(|e| match e {
                    lexopt::Error::MissingValue { .. } => {
                        fail(out, default_workers, "flag needs an argument: -ext")
                    }
                    _ => fail(out, default_workers, e.to_string()),
                })?;
                extensions_raw = Some(val.to_string_lossy().into_owned());
            }
            Short('j') | Long("jobs") | Long("workers") | Long("j") => {
                let val = parser.value().map_err(|e| match e {
                    lexopt::Error::MissingValue { .. } => {
                        fail(out, default_workers, "flag needs an argument: -j")
                    }
                    _ => fail(out, default_workers, e.to_string()),
                })?;
                let val_str = val.to_string_lossy();
                match val_str.parse::<i64>() {
                    Ok(v) => workers_raw = Some(v),
                    Err(error) => {
                        let reason = match error.kind() {
                            std::num::IntErrorKind::PosOverflow
                            | std::num::IntErrorKind::NegOverflow => "value out of range",
                            _ => "parse error",
                        };
                        return Err(fail(
                            out,
                            default_workers,
                            format!("invalid value {val_str:?} for flag -j: {reason}"),
                        ));
                    }
                }
            }
            Value(val) => {
                paths.push(val);
            }
            Short(c) => {
                return Err(fail(
                    out,
                    default_workers,
                    format!("flag provided but not defined: -{c}"),
                ));
            }
            Long(name) => {
                return Err(fail(
                    out,
                    default_workers,
                    format!("flag provided but not defined: -{name}"),
                ));
            }
        }
    }

    let workers = match workers_raw {
        Some(value) if value > 0 => usize::try_from(value).unwrap_or(usize::MAX),
        _ => default_workers,
    };

    let mut extensions = Vec::new();
    // if-let chain (edition 2024)
    if let Some(raw) = extensions_raw
        && !raw.trim().is_empty()
    {
        for entry in raw.split(',') {
            let entry = entry.trim().to_lowercase();
            if entry.is_empty() {
                continue;
            }
            if entry.starts_with('.') {
                extensions.push(entry);
            } else {
                extensions.push(format!(".{entry}"));
            }
        }
    }

    Ok(Config {
        extensions,
        workers,
        json_output,
        show_version: version_flag || v_flag,
        by_email,
        ignore_blank,
        committed_only,
        ascii,
        no_progress,
        paths,
    })
}

/// Normalizes raw command-line arguments so both GNU long options (`--flag`),
/// GNU short options (`-h`, `-v`, `-j`, `-e`, `-w`, `-c`, `-a`, `-n`, `-x`),
/// and legacy single-dash long options (`-json`, `-email`, etc.) parse cleanly.
fn normalize_args(args: &[OsString]) -> Vec<OsString> {
    const BOOL_SHORT_FLAGS: &[char] = &['h', 'v', 'e', 'w', 'c', 'a', 'n'];

    let mut normalized = Vec::with_capacity(args.len());
    let mut in_separator = false;

    for arg in args {
        if in_separator {
            normalized.push(arg.clone());
            continue;
        }

        let s = arg.to_string_lossy();
        if s == "--" {
            in_separator = true;
            normalized.push(arg.clone());
            continue;
        }

        if s == "-" || s.starts_with("--") || !s.starts_with('-') {
            normalized.push(arg.clone());
            continue;
        }

        // Starts with single '-' and has length >= 2
        let stripped = &s[1..];
        let (name, inline) = stripped.split_once('=').unwrap_or((stripped, ""));

        // Short flag with inline = (e.g. -v=false, -ascii=false): convert to --v=false, --ascii=false
        if !inline.is_empty() {
            normalized.push(OsString::from(format!("-{s}")));
            continue;
        }

        // Standalone single-character short flag (e.g. -h, -v, -j, -e, -w, -c, -a, -n, -x)
        if name.len() == 1 {
            normalized.push(arg.clone());
            continue;
        }

        // Short flag with attached numeric or dot value (e.g. -j4, -x.rs)
        if (s.starts_with("-j") && s.chars().nth(2).is_some_and(|c| c.is_ascii_digit()))
            || s.starts_with("-x.")
        {
            normalized.push(arg.clone());
            continue;
        }

        // Clustered short flags consisting entirely of bool flags (e.g. -ac, -ew, -acn)
        if name.chars().all(|c| BOOL_SHORT_FLAGS.contains(&c)) {
            normalized.push(arg.clone());
            continue;
        }

        // Legacy single-dash long flag (e.g. -json, -email, -nope, etc.).
        // Transform to double dash: -json -> --json
        normalized.push(OsString::from(format!("-{s}")));
    }

    normalized
}

fn parse_bool_opt(
    val: Option<OsString>,
    flag: &str,
    out: &mut dyn Write,
    default_workers: usize,
) -> Result<bool, ParseError> {
    match val {
        None => Ok(true),
        Some(val) => {
            let s = val.to_string_lossy();
            match s.as_ref() {
                "1" | "t" | "T" | "true" | "TRUE" | "True" => Ok(true),
                "0" | "f" | "F" | "false" | "FALSE" | "False" => Ok(false),
                _ => Err(fail(
                    out,
                    default_workers,
                    format!("invalid boolean value {s:?} for -{flag}: parse error"),
                )),
            }
        }
    }
}

fn fail(out: &mut dyn Write, default_workers: usize, message: impl Into<String>) -> ParseError {
    let message = message.into();
    let _ = writeln!(out, "error: {message}");
    write_help(out, default_workers);
    ParseError::Message(message)
}

pub fn help_text(default_workers: usize) -> String {
    format!(
        "blamr: concurrent git-blame author attribution and line statistics\n\
         \n\
         Usage:\n\
         \x20 blamr [flags] [paths...]\n\
         \x20 git blamr [flags] [paths...]\n\
         \n\
         Flags:\n\
         \x20 -a, --ascii\n\
         \x20   \tuse ASCII table and progress output\n\
         \x20 -c, --committed-only\n\
         \x20   \texclude uncommitted working tree changes from attribution\n\
         \x20 -e, --email\n\
         \x20   \taggregate lines by author email instead of name\n\
         \x20 -x, --ext string\n\
         \x20   \tcomma-separated extensions to filter (e.g. .typ,.tex,.rs). Defaults to all text files.\n\
         \x20 -h, --help\n\
         \x20   \tprint this help and exit\n\
         \x20 -w, --ignore-blank\n\
         \x20   \tignore empty and whitespace-only lines\n\
         \x20 -j, --jobs int\n\
         \x20   \tnumber of concurrent git-blame workers (default {default_workers})\n\
         \x20     --json\n\
         \x20   \toutput results as structured JSON\n\
         \x20 -n, --no-progress\n\
         \x20   \tdisable progress output\n\
         \x20 -v, --version\n\
         \x20   \tprint version information and exit\n"
    )
}

fn write_help(out: &mut dyn Write, default_workers: usize) {
    let _ = out.write_all(help_text(default_workers).as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn defaults() {
        let mut out = Vec::new();
        let cfg = parse(&[], 4, &mut out).unwrap();
        assert_eq!(cfg.workers, 4);
        assert!(!cfg.show_version);
        assert!(!cfg.json_output);
        assert!(!cfg.by_email);
        assert!(!cfg.ignore_blank);
        assert!(!cfg.committed_only);
        assert!(!cfg.ascii);
        assert!(!cfg.no_progress);
        assert!(cfg.extensions.is_empty());
        assert!(cfg.paths.is_empty());
    }

    #[test]
    fn output_flags_parse() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["-ascii", "-no-progress"]), 4, &mut out).unwrap();
        assert!(cfg.ascii);
        assert!(cfg.no_progress);

        let cfg = parse(&args(&["-ascii=false"]), 4, &mut out).unwrap();
        assert!(!cfg.ascii);

        // Also test GNU long flags with double dash
        let cfg = parse(&args(&["--ascii", "--no-progress"]), 4, &mut out).unwrap();
        assert!(cfg.ascii);
        assert!(cfg.no_progress);

        let cfg = parse(&args(&["--ascii=false"]), 4, &mut out).unwrap();
        assert!(!cfg.ascii);
    }

    #[test]
    fn extensions_are_normalized() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["-ext", "rs, .TS, typ , "]), 2, &mut out).unwrap();
        assert_eq!(cfg.extensions, vec![".rs", ".ts", ".typ"]);

        let cfg = parse(&args(&["--ext", "rs, .TS, typ , "]), 2, &mut out).unwrap();
        assert_eq!(cfg.extensions, vec![".rs", ".ts", ".typ"]);

        let cfg = parse(&args(&["-x", "rs, .TS, typ , "]), 2, &mut out).unwrap();
        assert_eq!(cfg.extensions, vec![".rs", ".ts", ".typ"]);
    }

    #[test]
    fn flags_after_paths() {
        let mut out = Vec::new();
        let cfg = parse(
            &args(&["src/auth", "-json", "-ext", ".rs", "pkg/api"]),
            4,
            &mut out,
        )
        .unwrap();
        assert!(cfg.json_output);
        assert_eq!(cfg.extensions, vec![".rs"]);
        assert_eq!(cfg.paths, args(&["src/auth", "pkg/api"]));

        // With GNU double dash flags
        let cfg = parse(
            &args(&["src/auth", "--json", "--ext", ".rs", "pkg/api"]),
            4,
            &mut out,
        )
        .unwrap();
        assert!(cfg.json_output);
        assert_eq!(cfg.extensions, vec![".rs"]);
        assert_eq!(cfg.paths, args(&["src/auth", "pkg/api"]));
    }

    #[test]
    fn repeated_values_last_wins() {
        let mut out = Vec::new();
        let cfg = parse(
            &args(&["-j", "2", "-j", "7", "-ext", ".rs", "-ext", ".ts"]),
            4,
            &mut out,
        )
        .unwrap();
        assert_eq!(cfg.workers, 7);
        assert_eq!(cfg.extensions, vec![".ts"]);

        let cfg = parse(&args(&["--jobs", "3", "--jobs", "8"]), 4, &mut out).unwrap();
        assert_eq!(cfg.workers, 8);
    }

    #[test]
    fn non_positive_workers_fall_back() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["-j", "0"]), 4, &mut out).unwrap();
        assert_eq!(cfg.workers, 4);
        let cfg = parse(&args(&["-j", "-3"]), 0, &mut out).unwrap();
        assert_eq!(cfg.workers, 1);

        let cfg = parse(&args(&["--jobs", "0"]), 4, &mut out).unwrap();
        assert_eq!(cfg.workers, 4);
    }

    #[test]
    fn version_flags() {
        let mut out = Vec::new();
        assert!(parse(&args(&["-v"]), 4, &mut out).unwrap().show_version);
        assert!(
            parse(&args(&["-version"]), 4, &mut out)
                .unwrap()
                .show_version
        );
        assert!(
            parse(&args(&["--version"]), 4, &mut out)
                .unwrap()
                .show_version
        );
        assert!(
            parse(&args(&["-version", "-v=false"]), 4, &mut out)
                .unwrap()
                .show_version
        );
        assert!(
            parse(&args(&["-v", "-version=false"]), 4, &mut out)
                .unwrap()
                .show_version
        );
        assert!(
            !parse(&args(&["-version", "-version=false"]), 4, &mut out)
                .unwrap()
                .show_version
        );
    }

    #[test]
    fn worker_overflow_reports_range_error() {
        let mut out = Vec::new();
        let err = parse(&args(&["-j=999999999999999999999999"]), 4, &mut out).unwrap_err();
        assert_eq!(
            err,
            ParseError::Message(
                "invalid value \"999999999999999999999999\" for flag -j: value out of range"
                    .to_owned()
            )
        );
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with(
            "error: invalid value \"999999999999999999999999\" for flag -j: value out of range\nblamr:"
        ));
        assert!(text.contains("Usage:"));
    }

    #[test]
    fn invalid_bool_reports_value() {
        let mut out = Vec::new();
        let err = parse(&args(&["-json=abc"]), 4, &mut out).unwrap_err();
        assert_eq!(
            err,
            ParseError::Message("invalid boolean value \"abc\" for -json: parse error".to_owned())
        );
        assert!(
            String::from_utf8(out)
                .unwrap()
                .starts_with("error: invalid boolean value \"abc\" for -json: parse error\n")
        );
    }

    #[test]
    fn help_is_reported() {
        let mut out = Vec::new();
        let err = parse(&args(&["-h"]), 4, &mut out).unwrap_err();
        assert_eq!(err, ParseError::Help);
        assert!(out.is_empty());

        let err = parse(&args(&["--help"]), 4, &mut out).unwrap_err();
        assert_eq!(err, ParseError::Help);
        assert!(out.is_empty());

        assert!(help_text(4).contains("Usage:"));
    }

    #[test]
    fn separator_stops_flag_parsing() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["--", "-json", "src"]), 4, &mut out).unwrap();
        assert!(!cfg.json_output);
        assert_eq!(cfg.paths, args(&["-json", "src"]));

        let cfg = parse(&args(&["--", "--json", "src"]), 4, &mut out).unwrap();
        assert!(!cfg.json_output);
        assert_eq!(cfg.paths, args(&["--json", "src"]));
    }

    #[test]
    fn bool_flags_do_not_consume_next_argument() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["-json", "false"]), 4, &mut out).unwrap();
        assert!(cfg.json_output);
        assert_eq!(cfg.paths, args(&["false"]));

        let cfg = parse(&args(&["--json", "false"]), 4, &mut out).unwrap();
        assert!(cfg.json_output);
        assert_eq!(cfg.paths, args(&["false"]));
    }

    #[test]
    fn gnu_short_flags_and_clustering() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["-acn"]), 4, &mut out).unwrap();
        assert!(cfg.ascii);
        assert!(cfg.committed_only);
        assert!(cfg.no_progress);

        let cfg = parse(&args(&["-e", "-w"]), 4, &mut out).unwrap();
        assert!(cfg.by_email);
        assert!(cfg.ignore_blank);
    }

    #[test]
    fn undefined_flags_and_missing_arguments() {
        let mut out = Vec::new();
        let _ = parse(&args(&["-z"]), 4, &mut out);
        assert!(String::from_utf8_lossy(&out).contains("flag provided but not defined: -z"));

        out.clear();
        let _ = parse(&args(&["--unknown-flag"]), 4, &mut out);
        assert!(
            String::from_utf8_lossy(&out).contains("flag provided but not defined: -unknown-flag")
        );

        out.clear();
        let _ = parse(&args(&["-ext"]), 4, &mut out);
        assert!(String::from_utf8_lossy(&out).contains("flag needs an argument: -ext"));

        out.clear();
        let _ = parse(&args(&["-j"]), 4, &mut out);
        assert!(String::from_utf8_lossy(&out).contains("flag needs an argument: -j"));
    }

    #[test]
    fn attached_flag_values_and_optional_bools() {
        let mut out = Vec::new();
        let cfg = parse(
            &args(&[
                "-j8",
                "-x.rs",
                "--email=true",
                "--ignore-blank=false",
                "--committed-only=true",
                "--ascii=false",
                "--no-progress=true",
            ]),
            4,
            &mut out,
        )
        .unwrap();
        assert_eq!(cfg.workers, 8);
        assert_eq!(cfg.extensions, vec![".rs"]);
        assert!(cfg.by_email);
        assert!(!cfg.ignore_blank);
        assert!(cfg.committed_only);
        assert!(!cfg.ascii);
        assert!(cfg.no_progress);
    }
}
