use crate::config::Config;
use crate::diff_overlay;
use crate::git::{TrackedPath, open_repository, path_to_git_string};
use crate::stats::Stats;
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use gix::bstr::ByteSlice;
use std::borrow::Cow;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread;

pub enum BlameEvent<'a> {
    Progress {
        done: usize,
        total: usize,
        path: &'a str,
    },
    Warning {
        path: &'a str,
        message: &'a str,
    },
}

struct Event {
    display_path: String,
    result: Result<HashMap<String, u64>, String>,
}

pub fn blame_files<F>(
    config: &Config,
    files: Vec<TrackedPath>,
    mut on_event: F,
) -> Result<Stats, String>
where
    F: FnMut(BlameEvent<'_>),
{
    let total = files.len();
    if total == 0 {
        return Ok(Stats::empty());
    }

    let repo = open_repository()?;
    let sync_repo = repo.into_sync();

    let workers = config.workers.max(1).min(total);
    let job_capacity = workers.saturating_mul(2).max(1);
    let (job_tx, job_rx) = bounded::<TrackedPath>(job_capacity);
    let (event_tx, event_rx) = unbounded::<Event>();

    let mut handles = Vec::with_capacity(workers);
    for index in 0..workers {
        let jobs = job_rx.clone();
        let events = event_tx.clone();
        let config = config.clone();
        let sync_repo = sync_repo.clone();
        let handle = thread::Builder::new()
            .name(format!("blamr-worker-{index}"))
            .spawn(move || worker_loop(&sync_repo, &config, &jobs, &events))
            .map_err(|error| format!("cannot start worker thread: {error}"))?;
        handles.push(handle);
    }
    drop(event_tx);

    let feeder = thread::Builder::new()
        .name("blamr-feeder".to_owned())
        .spawn(move || {
            for file in files {
                if job_tx.send(file).is_err() {
                    break;
                }
            }
        })
        .map_err(|error| format!("cannot start feeder thread: {error}"))?;
    drop(job_rx);

    let mut counts: HashMap<String, u64> = HashMap::new();
    let mut succeeded: u64 = 0;

    for (done, event) in event_rx.into_iter().enumerate() {
        let done = done + 1;
        on_event(BlameEvent::Progress {
            done,
            total,
            path: &event.display_path,
        });

        match event.result {
            Ok(file_counts) => {
                succeeded += 1;
                for (author, lines) in file_counts {
                    *counts.entry(author).or_default() += lines;
                }
            }
            Err(error) => {
                on_event(BlameEvent::Warning {
                    path: &event.display_path,
                    message: &error,
                });
            }
        }
    }

    let _ = feeder.join();
    for handle in handles {
        if handle.join().is_err() {
            eprintln!("warning: a blame worker thread terminated unexpectedly");
        }
    }

    Ok(Stats::from_counts(counts, succeeded))
}

/// Per-worker HEAD state, shared across all files processed by one thread.
struct HeadState<'a> {
    commit: Option<&'a gix::Commit<'a>>,
    tree: Option<&'a gix::Tree<'a>>,
}

fn worker_loop(
    sync_repo: &gix::ThreadSafeRepository,
    config: &Config,
    jobs: &Receiver<TrackedPath>,
    events: &Sender<Event>,
) {
    let repo = sync_repo.to_thread_local();
    let mailmap = repo.open_mailmap();
    let normalize_crlf = should_normalize_crlf(&repo);
    let head_commit = repo.head_commit().ok();
    let head_tree = head_commit.as_ref().and_then(|c| c.tree().ok());
    let head = HeadState {
        commit: head_commit.as_ref(),
        tree: head_tree.as_ref(),
    };
    let mut author_cache: HashMap<gix::ObjectId, String> = HashMap::new();

    while let Ok(file) = jobs.recv() {
        let display_path = file.display_path.clone();

        // SAFETY: author_cache is per-thread and not observed by any other thread.
        // After a panic inside blame_worktree, author_cache may be partially updated,
        // but it is never accessed outside this thread, so corrupted state cannot
        // escape. The worker loop continues processing subsequent files normally.
        let result = catch_unwind(AssertUnwindSafe(|| {
            blame_worktree(
                &repo,
                &mailmap,
                &head,
                &mut author_cache,
                config,
                &file,
                normalize_crlf,
            )
        }))
        .unwrap_or_else(|_| Err("internal worker panic".to_owned()));

        if events
            .send(Event {
                display_path,
                result,
            })
            .is_err()
        {
            break;
        }
    }
}

fn blame_worktree(
    repo: &gix::Repository,
    mailmap: &gix::mailmap::Snapshot,
    head: &HeadState<'_>,
    author_cache: &mut HashMap<gix::ObjectId, String>,
    config: &Config,
    file: &TrackedPath,
    normalize_crlf_enabled: bool,
) -> Result<HashMap<String, u64>, String> {
    let raw_contents = file
        .read_worktree()
        .map_err(|error| format!("cannot read worktree file: {error}"))?;

    if raw_contents.is_empty() {
        return Ok(HashMap::new());
    }

    let contents: Cow<'_, [u8]> = if normalize_crlf_enabled {
        normalize_crlf(&raw_contents)
    } else {
        Cow::Borrowed(&raw_contents)
    };

    let git_path_str = path_to_git_string(&file.relative_path);
    let head_entry = head
        .tree
        .and_then(|tree| tree.lookup_entry_by_path(&git_path_str).ok().flatten());

    let mut counts = HashMap::new();

    let Some(head_entry) = head_entry else {
        if !config.committed_only {
            let lines = split_lines(&contents);
            let count = lines
                .iter()
                .filter(|line| !config.ignore_blank || !is_blank(line))
                .count() as u64;
            if count > 0 {
                counts.insert(uncommitted_author(config).to_owned(), count);
            }
        }
        return Ok(counts);
    };

    let head_commit = head.commit.unwrap();
    let head_object = head_entry
        .object()
        .map_err(|error| format!("cannot read HEAD blob: {error}"))?;
    let head_blob = head_object.into_blob();
    let head_bytes = head_blob.data.as_slice();

    let outcome = repo
        .blame_file(
            gix::bstr::BStr::new(git_path_str.as_bytes()),
            head_commit.id,
            Default::default(),
        )
        .map_err(|error| format!("git blame failed: {error}"))?;

    let resolve_author =
        |commit_id: gix::ObjectId, author_cache: &mut HashMap<gix::ObjectId, String>| -> String {
            if let Some(author) = author_cache.get(&commit_id) {
                return author.clone();
            }
            let author = match repo.find_object(commit_id) {
                Ok(obj) => {
                    let commit = obj.into_commit();
                    let sig = commit.author().unwrap_or_default();
                    let resolved = mailmap.resolve(sig);
                    if config.by_email {
                        String::from_utf8_lossy(resolved.email.trim()).to_lowercase()
                    } else {
                        String::from_utf8_lossy(resolved.name.trim()).into_owned()
                    }
                }
                Err(_) => uncommitted_author(config).to_owned(),
            };
            let author = if author.is_empty() {
                uncommitted_author(config).to_owned()
            } else {
                author
            };
            author_cache.insert(commit_id, author.clone());
            author
        };

    if contents == head_bytes {
        let lines = split_lines(&contents);
        for entry in &outcome.entries {
            let author = resolve_author(entry.commit_id, author_cache);
            let start = entry.start_in_blamed_file as usize;
            for offset in 0..(entry.len.get() as usize) {
                if let Some(line) = lines.get(start + offset) {
                    if config.ignore_blank && is_blank(line) {
                        continue;
                    }
                    *counts.entry(author.clone()).or_default() += 1;
                }
            }
        }
        return Ok(counts);
    }

    let lines = split_lines(&contents);
    let mapping = diff_overlay::map_worktree_to_head(head_bytes, &contents);

    let head_line_count = count_lines(head_bytes);
    let mut head_line_authors: Vec<Option<String>> = vec![None; head_line_count];
    for entry in &outcome.entries {
        let author = resolve_author(entry.commit_id, author_cache);
        let start = entry.start_in_blamed_file as usize;
        for offset in 0..(entry.len.get() as usize) {
            if let Some(slot) = head_line_authors.get_mut(start + offset) {
                *slot = Some(author.clone());
            }
        }
    }

    for (line_idx, line) in lines.iter().enumerate() {
        if config.ignore_blank && is_blank(line) {
            continue;
        }

        let head_line = mapping.worktree_to_head.get(line_idx).copied().flatten();
        match head_line {
            Some(head_idx) => {
                let author = head_line_authors
                    .get(head_idx)
                    .and_then(Option::as_ref)
                    .cloned()
                    .unwrap_or_else(|| uncommitted_author(config).to_owned());
                *counts.entry(author).or_default() += 1;
            }
            None => {
                if !config.committed_only {
                    *counts
                        .entry(uncommitted_author(config).to_owned())
                        .or_default() += 1;
                }
            }
        }
    }

    Ok(counts)
}

fn should_normalize_crlf(repo: &gix::Repository) -> bool {
    let config = repo.config_snapshot();
    if config.boolean("core.autocrlf").unwrap_or(false) {
        return true;
    }
    config
        .string("core.autocrlf")
        .map(|val| val.as_bytes().eq_ignore_ascii_case(b"input"))
        .unwrap_or(false)
}

fn normalize_crlf(contents: &[u8]) -> Cow<'_, [u8]> {
    // Fast path: if there are no CRLF sequences, return a zero-copy borrow.
    if !contents.windows(2).any(|w| w == b"\r\n") {
        return Cow::Borrowed(contents);
    }
    let mut normalized = Vec::with_capacity(contents.len());
    let mut index = 0;
    while index < contents.len() {
        if contents[index] == b'\r' && contents.get(index + 1) == Some(&b'\n') {
            normalized.push(b'\n');
            index += 2;
        } else {
            normalized.push(contents[index]);
            index += 1;
        }
    }
    Cow::Owned(normalized)
}

fn uncommitted_author(config: &Config) -> &'static str {
    if config.by_email {
        "not.committed.yet"
    } else {
        "Not Committed Yet"
    }
}

fn split_lines(contents: &[u8]) -> Vec<&[u8]> {
    if contents.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&[u8]> = contents.split(|byte| *byte == b'\n').collect();
    if contents.ends_with(b"\n") {
        lines.pop();
    }
    lines
}

/// Count lines without allocating — used where only the count is needed.
fn count_lines(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 0;
    }
    let n = bytes.iter().filter(|&&b| b == b'\n').count();
    // A trailing newline does not add an extra logical line.
    if bytes.ends_with(b"\n") { n } else { n + 1 }
}

fn is_blank(line: &[u8]) -> bool {
    if line.iter().all(|byte| byte.is_ascii_whitespace()) {
        return true;
    }
    if line.is_ascii() {
        return false;
    }
    String::from_utf8_lossy(line).trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_crlf_only() {
        assert_eq!(&*normalize_crlf(b"a\r\nb\rc\n"), b"a\nb\rc\n");
        assert_eq!(&*normalize_crlf(b"no newline"), b"no newline");
        // Fast path: no CRLF → returns a borrowed slice (no allocation).
        assert!(matches!(
            normalize_crlf(b"no newline"),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn detects_blank_lines() {
        assert!(is_blank(b""));
        assert!(is_blank(b" \t\r"));
        assert!(!is_blank(b"x"));
        assert!(!is_blank(b" x "));
        assert!(is_blank("\u{00a0}".as_bytes()));
    }

    #[test]
    fn count_lines_matches_split_lines_len() {
        for sample in [
            &b""[..],
            &b"\n"[..],
            &b"single line"[..],
            &b"single line\n"[..],
            &b"line 1\nline 2"[..],
            &b"line 1\nline 2\n"[..],
            &b"a\n\nb\n"[..],
        ] {
            assert_eq!(
                count_lines(sample),
                split_lines(sample).len(),
                "failed for {:?}",
                std::str::from_utf8(sample)
            );
        }
    }

    #[test]
    fn stats_from_worker_counts_are_sorted() {
        let mut counts = HashMap::new();
        counts.insert("Alice".to_owned(), 2);
        counts.insert("Bob".to_owned(), 1);
        let stats = Stats::from_counts(counts, 2);
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.total_lines, 3);
        assert_eq!(stats.authors[0].author, "Alice");
    }
}
