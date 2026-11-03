#![allow(dead_code)]

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct TestRepo {
    dir: TempDir,
}

impl TestRepo {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = Self { dir };
        repo.git(&["init"]);
        repo.git(&["config", "user.name", "Test User"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo.git(&["config", "core.autocrlf", "false"]);
        repo
    }

    pub fn git(&self, args: &[&str]) -> std::process::Output {
        let output = std::process::Command::new("git")
            .current_dir(self.path())
            .args(args)
            .output()
            .expect("failed to execute git");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn workdir(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    pub fn write(&self, relative: &str, contents: impl AsRef<[u8]>) {
        let path = self.path().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    pub fn remove(&self, relative: &str) {
        fs::remove_file(self.path().join(relative)).unwrap();
    }

    pub fn stage(&self, relative: &str) {
        self.git(&["add", "--", relative]);
    }

    pub fn unstage(&self, relative: &str) {
        self.git(&["rm", "--cached", "--", relative]);
    }

    pub fn stage_all(&self) {
        self.git(&["add", "-A"]);
    }

    pub fn commit(&self, name: &str, email: &str, message: &str) {
        self.stage_all();
        let name_arg = format!("user.name={name}");
        let email_arg = format!("user.email={email}");
        let output = std::process::Command::new("git")
            .current_dir(self.path())
            .args(&["-c", &name_arg, "-c", &email_arg, "commit", "-m", message])
            .output()
            .expect("failed to execute git commit");
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    pub fn commit_file(
        &self,
        relative: &str,
        contents: impl AsRef<[u8]>,
        name: &str,
        email: &str,
        message: &str,
    ) {
        self.write(relative, contents);
        self.stage(relative);
        self.commit(name, email, message)
    }

    pub fn set_autocrlf(&self, value: bool) {
        self.git(&[
            "config",
            "core.autocrlf",
            if value { "true" } else { "false" },
        ]);
    }
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run(repo: &TestRepo, args: &[&str]) -> std::process::Output {
    Command::cargo_bin("blamr")
        .unwrap()
        .current_dir(repo.path())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .args(args)
        .output()
        .unwrap()
}

pub fn run_at(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::cargo_bin("blamr")
        .unwrap()
        .current_dir(cwd)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .args(args)
        .output()
        .unwrap()
}

pub fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

pub fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

pub fn json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

pub fn assert_success(output: &std::process::Output) {
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(output));
}
