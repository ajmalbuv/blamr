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
        Err(ParseError::Help) => return 0,
        Err(ParseError::Message(message)) => {
            let _ = writeln!(stderr, "error: {message}");
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

    let is_tty = io::stdout().is_terminal();
    let json_output = config.json_output;

    let stats = match blame::blame_files(&config, files, |done, total, file| {
        if !json_output {
            let mut stdout = io::stdout();
            let _ = output::print_progress(&mut stdout, done, total, file, is_tty);
        }
    }) {
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
    match output::print_table(&mut stdout, &stats) {
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
    error.kind() == io::ErrorKind::BrokenPipe || error.raw_os_error() == Some(32)
}
