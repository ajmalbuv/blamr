mod blame;
mod config;
mod filter;
mod git;
mod output;
mod stats;

use config::ParseError;
use std::io::{self, IsTerminal, Write};

pub fn run() -> i32 {
    let default_workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);

    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut stderr = io::stderr().lock();
    let config = match config::parse(&args, default_workers, &mut stderr) {
        Ok(config) => config,
        Err(ParseError::Help) => {
            drop(stderr);
            return print_help(default_workers);
        }
        Err(ParseError::Message(_)) => {
            return 2;
        }
    };
    drop(stderr);

    if config.show_version {
        return print_line(&format!("blamr version {}", output::version_string()));
    }

    let files = match git::collect_tracked(&config) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };

    let stdout_tty = io::stdout().is_terminal();
    let stderr_tty = io::stderr().is_terminal();
    let ascii = config.ascii || !output::unicode_supported(stdout_tty);
    let progress_enabled = stderr_tty && !config.json_output && !config.no_progress;
    let mut progress = output::Progress::new(progress_enabled, output::terminal_width());
    let mut stderr = io::stderr().lock();

    let result = blame::blame_files(&config, files, |event| match event {
        blame::BlameEvent::Progress { done, total, path } => {
            let _ = progress.update(&mut stderr, done, total, path);
        }
        blame::BlameEvent::Warning { path, message } => {
            let _ = progress.clear(&mut stderr);
            let _ = writeln!(
                stderr,
                "warning: blame {}: {}",
                output::sanitize_terminal(path),
                output::sanitize_terminal(message)
            );
        }
    });
    let _ = progress.finish(&mut stderr);
    drop(stderr);

    let stats = match result {
        Ok(stats) => stats,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };

    if stats.total_files == 0 {
        if config.json_output {
            return print_json_result(&stats);
        }

        let message = if !config.extensions.is_empty() {
            format!(
                "no matching files ({}) tracked in this repo",
                config.extensions.join(", ")
            )
        } else {
            "no text files found tracked in this repo".to_owned()
        };
        return print_line(&message);
    }

    if config.json_output {
        return print_json_result(&stats);
    }

    let mut stdout = io::stdout();
    match output::print_table(
        &mut stdout,
        &stats,
        output::TableOptions {
            ascii,
            by_email: config.by_email,
        },
    ) {
        Ok(()) => 0,
        Err(error) if is_broken_pipe(&error) => 0,
        Err(error) => {
            eprintln!("error: cannot write output: {error}");
            1
        }
    }
}

fn print_help(default_workers: usize) -> i32 {
    let mut stdout = io::stdout();
    match stdout.write_all(config::help_text(default_workers).as_bytes()) {
        Ok(()) => 0,
        Err(error) if is_broken_pipe(&error) => 0,
        Err(error) => {
            eprintln!("error: cannot write output: {error}");
            1
        }
    }
}

fn print_line(message: &str) -> i32 {
    let mut stdout = io::stdout();
    match writeln!(stdout, "{message}") {
        Ok(()) => 0,
        Err(error) if is_broken_pipe(&error) => 0,
        Err(error) => {
            eprintln!("error: cannot write output: {error}");
            1
        }
    }
}

fn print_json_result(stats: &stats::Stats) -> i32 {
    let mut stdout = io::stdout();
    match output::print_json(&mut stdout, stats) {
        Ok(()) => 0,
        Err(error) if is_broken_pipe(&error) => 0,
        Err(error) => {
            eprintln!("error serializing JSON: {error}");
            1
        }
    }
}

fn is_broken_pipe(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::BrokenPipe
}
