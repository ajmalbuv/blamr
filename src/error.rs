use std::io;
use thiserror::Error;

/// Errors that can occur when blaming an individual file in a worker thread.
#[derive(Debug, Error)]
pub enum BlameFileError {
    #[error("cannot read worktree file: {0}")]
    WorktreeRead(#[source] io::Error),

    #[error("cannot read HEAD blob: {0}")]
    HeadBlob(#[source] gix::Error),

    #[error("git blame failed: {0}")]
    GitBlame(#[source] gix::Error),

    #[error("internal worker panic")]
    WorkerPanic,
}

/// Errors that can occur while initializing threads in `blame_files`.
#[derive(Debug, Error)]
pub enum BlameError {
    #[error("cannot start worker thread: {0}")]
    WorkerSpawn(#[source] io::Error),

    #[error("cannot start feeder thread: {0}")]
    FeederSpawn(#[source] io::Error),
}

/// Errors that can occur during repository discovery and tracked file collection.
#[derive(Debug, Error)]
pub enum GitError {
    #[error("cannot determine CWD: {0}")]
    Cwd(#[source] io::Error),

    #[error("not a git repo (or repository discovery failed): {0}")]
    Discovery(#[source] gix::Error),

    #[error("not a git repo (or repository has no worktree)")]
    NoWorktree,

    #[error("cannot open repository index: {0}")]
    IndexOpen(#[source] gix::Error),

    #[error("path is outside repository: {0}")]
    PathOutsideRepo(String),
}

/// Unified operational error enum for `run()`.
#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    Git(#[from] GitError),

    #[error(transparent)]
    Blame(#[from] BlameError),

    #[error("cannot write output: {0}")]
    Output(#[source] io::Error),

    #[error("error serializing JSON: {0}")]
    JsonOutput(#[source] io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn blame_file_error_formatting_and_sources() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = BlameFileError::WorktreeRead(io_err);
        assert_eq!(err.to_string(), "cannot read worktree file: file not found");
        assert!(err.source().is_some());

        let panic_err = BlameFileError::WorkerPanic;
        assert_eq!(panic_err.to_string(), "internal worker panic");
        assert!(panic_err.source().is_none());
    }

    #[test]
    fn git_error_formatting_and_sources() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let err = GitError::Cwd(io_err);
        assert_eq!(err.to_string(), "cannot determine CWD: access denied");
        assert!(err.source().is_some());

        let no_worktree = GitError::NoWorktree;
        assert_eq!(
            no_worktree.to_string(),
            "not a git repo (or repository has no worktree)"
        );
        assert!(no_worktree.source().is_none());

        let outside = GitError::PathOutsideRepo("foo/bar".to_owned());
        assert_eq!(outside.to_string(), "path is outside repository: foo/bar");
    }

    #[test]
    fn run_error_transparent_conversions() {
        let git_err = GitError::PathOutsideRepo("outside".to_owned());
        let run_err = RunError::from(git_err);
        assert_eq!(run_err.to_string(), "path is outside repository: outside");

        let io_err = io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe");
        let blame_err = BlameError::WorkerSpawn(io_err);
        let run_err = RunError::from(blame_err);
        assert_eq!(
            run_err.to_string(),
            "cannot start worker thread: broken pipe"
        );
    }
}
