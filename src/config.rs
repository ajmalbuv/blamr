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

    // Collect flags separately from positional arguments so flags may appear
    // after paths. Values for -ext and -j that begin with '-' are not consumed.
    let mut flag_args = Vec::new();
    let mut pos_args = Vec::new();
    let mut seen_separator = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        let text = arg.to_string_lossy();

        if text == "--" {
            seen_separator = true;
            pos_args.extend(args[i + 1..].iter().cloned());
            break;
        }

        if text.starts_with('-') {
            flag_args.push(arg.clone());
            if (text == "-ext" || text == "-j")
                && i + 1 < args.len()
                && !args[i + 1].to_string_lossy().starts_with('-')
            {
                i += 1;
                flag_args.push(args[i].clone());
            }
        } else {
            pos_args.push(arg.clone());
        }

        i += 1;
    }

    let mut ordered = flag_args;
    if seen_separator {
        ordered.push(OsString::from("--"));
    }
    ordered.extend(pos_args);

    let mut extensions_raw: Option<String> = None;
    let mut workers_raw: Option<i64> = None;
    let mut version_flag = false;
    let mut v_flag = false;
    let mut json_output = false;
    let mut by_email = false;
    let mut ignore_blank = false;
    let mut committed_only = false;
    let mut paths = Vec::new();

    let mut i = 0;
    while i < ordered.len() {
        let token = ordered[i].to_string_lossy().into_owned();

        if token == "--" {
            paths.extend(ordered[i + 1..].iter().cloned());
            break;
        }
        if token == "-" {
            paths.extend(ordered[i..].iter().cloned());
            break;
        }
        if !token.starts_with('-') {
            paths.extend(ordered[i..].iter().cloned());
            break;
        }

        let stripped = if let Some(rest) = token.strip_prefix("--") {
            rest
        } else {
            token.strip_prefix('-').unwrap_or(&token)
        };
        let (name, inline) = match stripped.split_once('=') {
            Some((name, value)) => (name, Some(value.to_owned())),
            None => (stripped, None),
        };

        match name {
            "h" | "help" => {
                write_help(out, default_workers);
                return Err(ParseError::Help);
            }
            "ext" => {
                let value = match inline {
                    Some(value) => value,
                    None => {
                        i += 1;
                        if i >= ordered.len() {
                            return Err(fail(out, default_workers, "flag needs an argument: -ext"));
                        }
                        ordered[i].to_string_lossy().into_owned()
                    }
                };
                extensions_raw = Some(value);
            }
            "j" => {
                let value = match inline {
                    Some(value) => value,
                    None => {
                        i += 1;
                        if i >= ordered.len() {
                            return Err(fail(out, default_workers, "flag needs an argument: -j"));
                        }
                        ordered[i].to_string_lossy().into_owned()
                    }
                };
                match value.parse::<i64>() {
                    Ok(value) => workers_raw = Some(value),
                    Err(error) => {
                        let reason = match error.kind() {
                            std::num::IntErrorKind::PosOverflow
                            | std::num::IntErrorKind::NegOverflow => "value out of range",
                            _ => "parse error",
                        };
                        return Err(fail(
                            out,
                            default_workers,
                            format!("invalid value {value:?} for flag -j: {reason}"),
                        ));
                    }
                }
            }
            "version" => match parse_bool_flag(inline.as_deref()) {
                Ok(value) => version_flag = value,
                Err(()) => {
                    return Err(fail(
                        out,
                        default_workers,
                        format!(
                            "invalid boolean value {:?} for -version: parse error",
                            inline.as_deref().unwrap_or("")
                        ),
                    ));
                }
            },
            "v" => match parse_bool_flag(inline.as_deref()) {
                Ok(value) => v_flag = value,
                Err(()) => {
                    return Err(fail(
                        out,
                        default_workers,
                        format!(
                            "invalid boolean value {:?} for -v: parse error",
                            inline.as_deref().unwrap_or("")
                        ),
                    ));
                }
            },
            "json" => match parse_bool_flag(inline.as_deref()) {
                Ok(value) => json_output = value,
                Err(()) => {
                    return Err(fail(
                        out,
                        default_workers,
                        format!(
                            "invalid boolean value {:?} for -json: parse error",
                            inline.as_deref().unwrap_or("")
                        ),
                    ));
                }
            },
            "email" => match parse_bool_flag(inline.as_deref()) {
                Ok(value) => by_email = value,
                Err(()) => {
                    return Err(fail(
                        out,
                        default_workers,
                        format!(
                            "invalid boolean value {:?} for -email: parse error",
                            inline.as_deref().unwrap_or("")
                        ),
                    ));
                }
            },
            "ignore-blank" => match parse_bool_flag(inline.as_deref()) {
                Ok(value) => ignore_blank = value,
                Err(()) => {
                    return Err(fail(
                        out,
                        default_workers,
                        format!(
                            "invalid boolean value {:?} for -ignore-blank: parse error",
                            inline.as_deref().unwrap_or("")
                        ),
                    ));
                }
            },
            "committed-only" => match parse_bool_flag(inline.as_deref()) {
                Ok(value) => committed_only = value,
                Err(()) => {
                    return Err(fail(
                        out,
                        default_workers,
                        format!(
                            "invalid boolean value {:?} for -committed-only: parse error",
                            inline.as_deref().unwrap_or("")
                        ),
                    ));
                }
            },
            _ => {
                return Err(fail(
                    out,
                    default_workers,
                    format!("flag provided but not defined: -{name}"),
                ));
            }
        }

        i += 1;
    }

    let workers = match workers_raw {
        Some(value) if value > 0 => usize::try_from(value).unwrap_or(usize::MAX),
        _ => default_workers,
    };

    let mut extensions = Vec::new();
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
        paths,
    })
}

fn fail(out: &mut dyn Write, default_workers: usize, message: impl Into<String>) -> ParseError {
    let message = message.into();
    let _ = writeln!(out, "{message}");
    write_help(out, default_workers);
    ParseError::Message(message)
}

fn parse_bool_flag(value: Option<&str>) -> Result<bool, ()> {
    match value {
        None => Ok(true),
        Some(value) => match value {
            "1" | "t" | "T" | "true" | "TRUE" | "True" => Ok(true),
            "0" | "f" | "F" | "false" | "FALSE" | "False" => Ok(false),
            _ => Err(()),
        },
    }
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
         \x20 -committed-only\n\
         \x20   \texclude uncommitted working tree changes from attribution\n\
         \x20 -email\n\
         \x20   \taggregate lines by author email instead of name\n\
         \x20 -ext string\n\
         \x20   \tcomma-separated extensions to filter (e.g. .typ,.tex,.rs). Defaults to all text files.\n\
         \x20 -ignore-blank\n\
         \x20   \tignore empty and whitespace-only lines\n\
         \x20 -j int\n\
         \x20   \tnumber of concurrent git-blame workers (default {default_workers})\n\
         \x20 -json\n\
         \x20   \toutput results as structured JSON\n\
         \x20 -v\tprint version information and exit (shorthand)\n\
         \x20 -version\n\
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
        assert!(cfg.extensions.is_empty());
        assert!(cfg.paths.is_empty());
    }

    #[test]
    fn extensions_are_normalized() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["-ext", "rs, .TS, typ , "]), 2, &mut out).unwrap();
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
    }

    #[test]
    fn non_positive_workers_fall_back() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["-j", "0"]), 4, &mut out).unwrap();
        assert_eq!(cfg.workers, 4);
        let cfg = parse(&args(&["-j", "-3"]), 0, &mut out).unwrap();
        assert_eq!(cfg.workers, 1);
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
            "invalid value \"999999999999999999999999\" for flag -j: value out of range\nblamr:"
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
                .starts_with("invalid boolean value \"abc\" for -json: parse error\n")
        );
    }

    #[test]
    fn help_is_reported() {
        let mut out = Vec::new();
        let err = parse(&args(&["-h"]), 4, &mut out).unwrap_err();
        assert_eq!(err, ParseError::Help);
        assert!(String::from_utf8(out).unwrap().contains("Usage:"));
    }

    #[test]
    fn separator_stops_flag_parsing() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["--", "-json", "src"]), 4, &mut out).unwrap();
        assert!(!cfg.json_output);
        assert_eq!(cfg.paths, args(&["-json", "src"]));
    }

    #[test]
    fn bool_flags_do_not_consume_next_argument() {
        let mut out = Vec::new();
        let cfg = parse(&args(&["-json", "false"]), 4, &mut out).unwrap();
        assert!(cfg.json_output);
        assert_eq!(cfg.paths, args(&["false"]));
    }
}
