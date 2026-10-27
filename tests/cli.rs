mod fixtures;

use assert_cmd::Command;
use fixtures::{TestRepo, assert_success, json, run, run_at, stderr, stdout};
use pretty_assertions::assert_eq;
#[test]
fn table_and_json_match_expected_contract() {
    let repo = TestRepo::new();
    repo.commit_file("a.txt", "one\ntwo\n", "Alice", "alice@example.com", "a");
    repo.commit_file("b.txt", "three\n", "Bob", "bob@example.com", "b");

    let output = run(&repo, &[]);
    assert_success(&output);
    let expected = "\n\n─────────────────────────────────────────────\nAuthor                       Lines        %\n─────────────────────────────────────────────\nAlice                            2    66.7%  █████████████\nBob                              1    33.3%  ██████\n─────────────────────────────────────────────\nTotal                            3\n";
    assert_eq!(stdout(&output), expected);
    assert_eq!(stderr(&output), "");

    let value = json(&run(&repo, &["-json"]));
    assert_eq!(value["total_files"], 2);
    assert_eq!(value["total_lines"], 3);
    assert_eq!(value["authors"][0]["author"], "Alice");
    assert_eq!(value["authors"][0]["lines"], 2);
    assert_eq!(value["authors"][0]["percentage"], 66.67);
    assert_eq!(value["authors"][1]["author"], "Bob");
    assert_eq!(value["authors"][1]["percentage"], 33.33);

    let value = json(&run(&repo, &["-json", "-email"]));
    assert_eq!(value["authors"][0]["author"], "alice@example.com");
    assert_eq!(value["authors"][1]["author"], "bob@example.com");
}

#[test]
fn worktree_edits_are_uncommitted_and_committed_only_excludes_them() {
    let repo = TestRepo::new();
    repo.commit_file(
        "file.txt",
        "one\ntwo\n",
        "Alice",
        "alice@example.com",
        "base",
    );
    repo.write("file.txt", "one\nchanged\n");

    let value = json(&run(&repo, &["-json"]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 2);
    assert_eq!(value["authors"][0]["author"], "Alice");
    assert_eq!(value["authors"][0]["lines"], 1);
    assert_eq!(value["authors"][1]["author"], "Not Committed Yet");
    assert_eq!(value["authors"][1]["lines"], 1);

    let value = json(&run(&repo, &["-json", "-committed-only"]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 1);
    assert_eq!(value["authors"][0]["author"], "Alice");

    let value = json(&run(&repo, &["-json", "-email", "-committed-only"]));
    assert_eq!(value["authors"][0]["author"], "alice@example.com");
}

#[test]
fn filters_apply_rules_and_extensions_bypass_them() {
    let repo = TestRepo::new();
    let lfs = b"version https://git-lfs.github.com/spec/v1\noid sha256:deadbeef\n";
    repo.commit_file("code.txt", "one\n", "Alice", "alice@example.com", "code");
    repo.commit_file(
        "utf16.txt",
        [0xFF, 0xFE, b'h', 0, b'i', 0],
        "Alice",
        "alice@example.com",
        "utf16",
    );
    repo.commit_file("empty.txt", "", "Alice", "alice@example.com", "empty");
    repo.commit_file("yarn.lock", "lock\n", "Alice", "alice@example.com", "lock");
    repo.commit_file("app.min.js", "min\n", "Alice", "alice@example.com", "min");
    repo.commit_file("notes.map", "map\n", "Alice", "alice@example.com", "map");
    repo.commit_file("image.svg", "svg\n", "Alice", "alice@example.com", "svg");
    repo.commit_file("bin.dat", b"a\0b\n", "Alice", "alice@example.com", "binary");
    repo.commit_file("lfs.txt", lfs, "Alice", "alice@example.com", "lfs");

    let value = json(&run(&repo, &["-json"]));
    assert_eq!(value["total_files"], 3);
    assert_eq!(value["total_lines"], 2);
    assert_eq!(value["authors"][0]["author"], "Alice");
    assert_eq!(value["authors"][0]["lines"], 2);

    let value = json(&run(&repo, &["-json", "-ext", ".js"]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 1);

    let value = json(&run(&repo, &["-json", "-ext", ".svg"]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 1);

    let value = json(&run(&repo, &["-json", "-ext", ".txt"]));
    assert_eq!(value["total_files"], 4);
    assert_eq!(value["total_lines"], 4);

    let output = run(&repo, &["-ext", ".rs"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "no matching files (.rs) tracked in this repo\n"
    );
}

#[test]
fn empty_files_succeed_but_all_filtered_files_report_no_files() {
    let repo = TestRepo::new();
    repo.commit_file("empty.txt", "", "Alice", "alice@example.com", "empty");

    let value = json(&run(&repo, &["-json"]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 0);
    assert_eq!(value["authors"], serde_json::json!([]));

    let output = run(&repo, &[]);
    assert_success(&output);
    assert!(stdout(&output).contains("Total                            0\n"));
    assert!(!stdout(&output).contains("no text files"));

    let filtered = TestRepo::new();
    filtered.commit_file("bin.dat", b"a\0b", "Alice", "alice@example.com", "binary");
    let output = run(&filtered, &[]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "no text files found tracked in this repo\n"
    );
}

#[test]
fn staged_new_and_no_head_files_are_handled() {
    let repo = TestRepo::new();
    repo.commit_file("base.txt", "base\n", "Alice", "alice@example.com", "base");
    repo.write("new.txt", "new\n");
    repo.stage("new.txt");

    let value = json(&run(&repo, &["-json"]));
    assert_eq!(value["total_files"], 2);
    assert_eq!(value["total_lines"], 2);
    assert_eq!(value["authors"][0]["author"], "Alice");
    assert_eq!(value["authors"][1]["author"], "Not Committed Yet");

    let empty_repo = TestRepo::new();
    empty_repo.write("new.txt", "new\n");
    empty_repo.stage("new.txt");
    let output = run(&empty_repo, &[]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "no text files found tracked in this repo\n"
    );
    assert!(stderr(&output).contains("warning: blame new.txt: cannot resolve HEAD:"));
}

#[test]
fn deleted_worktree_files_warn_and_do_not_count_as_successful_files() {
    let repo = TestRepo::new();
    repo.commit_file("gone.txt", "gone\n", "Alice", "alice@example.com", "base");
    repo.remove("gone.txt");

    let output = run(&repo, &[]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "no text files found tracked in this repo\n"
    );
    assert!(stderr(&output).contains("warning: blame gone.txt: cannot read worktree file:"));

    let value = json(&run(&repo, &["-json"]));
    assert_eq!(value["total_files"], 0);
    assert_eq!(value["total_lines"], 0);
    assert_eq!(value["authors"], serde_json::json!([]));
}

#[test]
fn paths_scope_spaces_absolute_paths_and_dash_files() {
    let repo = TestRepo::new();
    repo.commit_file("README.md", "root\n", "Alice", "alice@example.com", "root");
    repo.commit_file(
        "nested/space name.txt",
        "nested\n",
        "Alice",
        "alice@example.com",
        "nested",
    );
    repo.commit_file("-dash.txt", "dash\n", "Alice", "alice@example.com", "dash");
    repo.write("nested/untracked.txt", "untracked\n");

    let value = json(&run(&repo, &["-json"]));
    assert_eq!(value["total_files"], 3);

    let nested = repo.path().join("nested");
    let value = json(&run_at(&nested, &["-json"]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 1);

    let value = json(&run_at(&nested, &["-json", ".."]));
    assert_eq!(value["total_files"], 3);

    let absolute = repo.path().join("README.md");
    let absolute = absolute.to_str().unwrap();
    let value = json(&run_at(&nested, &["-json", absolute]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 1);

    let value = json(&run(&repo, &["-json", "--", "-dash.txt"]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 1);

    let value = json(&run(&repo, &["README.md", "-json"]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 1);

    let value = json(&run(&repo, &["-json", "missing.txt"]));
    assert_eq!(value["total_files"], 0);

    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("outside.txt");
    std::fs::write(&outside_file, "outside\n").unwrap();
    let output = run(&repo, &[outside_file.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("path is outside repository:"));
}

#[test]
fn blank_filtering_unicode_and_mailmap() {
    let repo = TestRepo::new();
    repo.commit_file(
        "blank.txt",
        "a\n\n   \nb\n",
        "Alice",
        "alice@example.com",
        "blank",
    );
    repo.commit_file(
        "héllo.txt",
        "unicode\n",
        "Ålice 🚀",
        "alice@example.com",
        "unicode",
    );
    repo.write(
        ".mailmap",
        "Carol Canon <carol@example.com> Alice <alice@example.com>\n",
    );

    let value = json(&run(&repo, &["-json"]));
    assert_eq!(value["total_lines"], 5);
    assert_eq!(value["authors"][0]["author"], "Carol Canon");
    assert_eq!(value["authors"][0]["lines"], 4);
    assert_eq!(value["authors"][1]["author"], "Ålice 🚀");
    assert_eq!(value["authors"][1]["lines"], 1);

    let value = json(&run(&repo, &["-json", "-ignore-blank"]));
    assert_eq!(value["total_lines"], 3);

    let value = json(&run(&repo, &["-json", "-email"]));
    assert_eq!(value["authors"][0]["author"], "carol@example.com");
}

#[test]
fn autocrlf_worktree_bytes_are_normalized_for_blame() {
    let repo = TestRepo::new();
    repo.set_autocrlf(true);
    repo.commit_file(
        "crlf.txt",
        "one\ntwo\n",
        "Alice",
        "alice@example.com",
        "base",
    );
    repo.write("crlf.txt", "one\r\ntwo\r\n");

    let value = json(&run(&repo, &["-json"]));
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 2);
    assert_eq!(value["authors"][0]["author"], "Alice");
    assert_eq!(value["authors"][0]["lines"], 2);
}

#[test]
fn help_version_parse_errors_and_both_binaries() {
    let repo = TestRepo::new();
    let output = run(&repo, &["-h"]);
    assert_success(&output);
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).starts_with(
        "blamr: concurrent git-blame author attribution and line statistics\n\nUsage:\n"
    ));

    for args in [
        &["-j=abc"][..],
        &["-json=abc"][..],
        &["-j=999999999999999999999999"][..],
        &["-nope"][..],
    ] {
        let output = run(&repo, args);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(stdout(&output), "");
        assert!(stderr(&output).contains("error:"));
    }

    let output = run(&repo, &["-version"]);
    assert_success(&output);
    assert!(stdout(&output).starts_with("blamr version "));
    assert!(stdout(&output).contains(" ("));
    let output = run(&repo, &["-version", "-v=false"]);
    assert_success(&output);
    assert!(stdout(&output).starts_with("blamr version "));

    let output = Command::cargo_bin("git-blamr")
        .unwrap()
        .arg("-version")
        .output()
        .unwrap();
    assert_success(&output);
    assert!(stdout(&output).starts_with("blamr version "));
}

#[test]
fn environment_overrides_and_bare_repository_behavior() {
    let repo = TestRepo::new();
    repo.commit_file(
        "file.txt",
        "one\ntwo\n",
        "Alice",
        "alice@example.com",
        "base",
    );
    let outside = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("blamr")
        .unwrap()
        .current_dir(outside.path())
        .env("GIT_DIR", repo.path().join(".git"))
        .env("GIT_WORK_TREE", repo.path())
        .env_remove("GIT_INDEX_FILE")
        .arg("-json")
        .output()
        .unwrap();
    assert_success(&output);
    let value = json(&output);
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 2);

    let bare = tempfile::tempdir().unwrap();
    git2::Repository::init_bare(bare.path()).unwrap();
    let output = Command::cargo_bin("blamr")
        .unwrap()
        .current_dir(bare.path())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("repository has no worktree"));
}

#[test]
fn works_without_git_on_path() {
    let repo = TestRepo::new();
    repo.commit_file("file.txt", "one\n", "Alice", "alice@example.com", "base");
    let output = Command::cargo_bin("blamr")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", "")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .arg("-json")
        .output()
        .unwrap();
    assert_success(&output);
    let value = json(&output);
    assert_eq!(value["total_files"], 1);
    assert_eq!(value["total_lines"], 1);
}

#[test]
fn missing_repository_is_a_fatal_error() {
    let dir = tempfile::tempdir().unwrap();
    let output = run_at(dir.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("not a git repo"));
}

#[test]
fn alternate_index_file_is_honored() {
    let repo = TestRepo::new();
    repo.commit_file("base.txt", "base\n", "Alice", "alice@example.com", "base");
    repo.write("alt.txt", "alt\n");
    repo.stage("alt.txt");
    let alt_index = repo.path().join(".git/alt-index");
    std::fs::copy(repo.path().join(".git/index"), &alt_index).unwrap();
    repo.unstage("alt.txt");

    assert_eq!(json(&run(&repo, &["-json"]))["total_files"], 1);
    let output = Command::cargo_bin("blamr")
        .unwrap()
        .current_dir(repo.path())
        .env("GIT_INDEX_FILE", &alt_index)
        .args(["-json"])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(json(&output)["total_files"], 2);
}
