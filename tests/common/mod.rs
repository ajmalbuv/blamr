#![allow(dead_code)]

use assert_cmd::Command;
use git2::{IndexAddOption, Oid, Repository, Signature};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct TestRepo {
    dir: TempDir,
    repo: Repository,
}

impl TestRepo {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
            config.set_bool("core.autocrlf", false).unwrap();
        }
        Self { dir, repo }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn workdir(&self) -> PathBuf {
        self.repo.workdir().unwrap().to_path_buf()
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
        let mut index = self.repo.index().unwrap();
        index.add_path(Path::new(relative)).unwrap();
        index.write().unwrap();
    }

    pub fn unstage(&self, relative: &str) {
        let mut index = self.repo.index().unwrap();
        index.remove_path(Path::new(relative)).unwrap();
        index.write().unwrap();
    }
    pub fn stage_all(&self) {
        let mut index = self.repo.index().unwrap();
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
    }

    pub fn commit(&self, name: &str, email: &str, message: &str) -> Oid {
        self.stage_all();
        let mut index = self.repo.index().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = self.repo.find_tree(tree_id).unwrap();
        let signature = Signature::now(name, email).unwrap();
        let parents = match self.repo.head() {
            Ok(head) => vec![head.peel_to_commit().unwrap()],
            Err(error)
                if matches!(
                    error.code(),
                    git2::ErrorCode::NotFound | git2::ErrorCode::UnbornBranch
                ) =>
            {
                Vec::new()
            }
            Err(error) => panic!("cannot read HEAD: {error}"),
        };
        let parent_refs: Vec<_> = parents.iter().collect();
        self.repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parent_refs,
            )
            .unwrap()
    }

    pub fn commit_file(
        &self,
        relative: &str,
        contents: impl AsRef<[u8]>,
        name: &str,
        email: &str,
        message: &str,
    ) -> Oid {
        self.write(relative, contents);
        self.stage(relative);
        self.commit(name, email, message)
    }

    pub fn set_autocrlf(&self, value: bool) {
        self.repo
            .config()
            .unwrap()
            .set_bool("core.autocrlf", value)
            .unwrap();
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
