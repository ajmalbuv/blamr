use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(blamr::run() as u8)
}
