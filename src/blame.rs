use crate::config::Config;
use crate::git::{TrackedPath, open_repository};
use crate::stats::Stats;
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use git2::{Blame, BlameOptions, ErrorCode, Oid, Repository};
use std::collections::HashMap;
use std::fs;
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

    let workers = config.workers.max(1).min(total);
    let job_capacity = workers.saturating_mul(2).max(1);
    let (job_tx, job_rx) = bounded::<TrackedPath>(job_capacity);
    let (event_tx, event_rx) = unbounded::<Event>();

    let mut handles = Vec::with_capacity(workers);
    for index in 0..workers {
        let jobs = job_rx.clone();
        let events = event_tx.clone();
        let config = config.clone();
        let handle = thread::Builder::new()
            .name(format!("blamr-worker-{index}"))
            .spawn(move || worker_loop(&config, &jobs, &events))
            .map_err(|error| format!("cannot start worker thread: {error}"))?;
        handles.push(handle);
    }
    drop(event_tx);

    for file in files {
        job_tx
            .send(file)
            .map_err(|error| format!("cannot queue blame job: {error}"))?;
    }
    drop(job_tx);
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

    for handle in handles {
        if handle.join().is_err() {
            eprintln!("warning: a blame worker thread terminated unexpectedly");
        }
    }

    Ok(Stats::from_counts(counts, succeeded))
}

fn worker_loop(config: &Config, jobs: &Receiver<TrackedPath>, events: &Sender<Event>) {
    let mut repository: Option<Repository> = None;
    let mut normalize_crlf: Option<bool> = None;

    while let Ok(file) = jobs.recv() {
        let display_path = file.display_path.clone();

        let result = catch_unwind(AssertUnwindSafe(|| {
            if repository.is_none() {
                let opened = open_repository()?;
                normalize_crlf = Some(should_normalize_crlf(&opened));
                repository = Some(opened);
            }
            blame_worktree(
                repository.as_ref().unwrap(),
                config,
                &file,
                normalize_crlf.unwrap_or(false),
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
    repository: &Repository,
    config: &Config,
    file: &TrackedPath,
    normalize_crlf_enabled: bool,
) -> Result<HashMap<String, u64>, String> {
    let contents = file
        .read_worktree()
        .map_err(|error| format!("cannot read worktree file: {error}"))?;

    let in_head = path_in_head(repository, &file.relative_path)?;

    if contents.is_empty() {
        return Ok(HashMap::new());
    }

    let lines = split_lines(&contents);
    let normalized;
    let buffer = if normalize_crlf_enabled {
        normalized = normalize_crlf(&contents);
        normalized.as_slice()
    } else {
        contents.as_slice()
    };
    let merge_oid = if file.conflicted {
        merge_head_oid(repository)
    } else {
        None
    };

    let merge_has_path = match merge_oid {
        Some(commit) => commit_has_path(repository, commit, &file.relative_path)?,
        None => false,
    };

    let mut counts = HashMap::new();

    if !in_head {
        if let Some(merge_oid) = merge_oid.filter(|_| merge_has_path) {
            let head_lines = vec![None; lines.len()];
            let merge_lines = line_authors_at(
                repository,
                &file.relative_path,
                buffer,
                Some(merge_oid),
                lines.len(),
                config,
            )?;
            count_conflict_lines(&lines, &head_lines, &merge_lines, config, &mut counts);
            return Ok(counts);
        }

        if !config.committed_only {
            let count = lines
                .iter()
                .filter(|line| {
                    !config.ignore_blank || !String::from_utf8_lossy(line).trim().is_empty()
                })
                .count() as u64;
            if count > 0 {
                counts.insert(uncommitted_author(config).to_owned(), count);
            }
        }
        return Ok(counts);
    }

    if let Some(merge_oid) = merge_oid.filter(|_| merge_has_path) {
        let head_lines = line_authors_at(
            repository,
            &file.relative_path,
            buffer,
            None,
            lines.len(),
            config,
        )?;
        let merge_lines = line_authors_at(
            repository,
            &file.relative_path,
            buffer,
            Some(merge_oid),
            lines.len(),
            config,
        )?;
        count_conflict_lines(&lines, &head_lines, &merge_lines, config, &mut counts);
        return Ok(counts);
    }

    let mut options = BlameOptions::new();
    options.ignore_whitespace(config.ignore_whitespace);
    options.use_mailmap(true);

    let blame = repository
        .blame_file(&file.relative_path, Some(&mut options))
        .map_err(|error| format!("git blame failed: {error}"))?;
    // `blame_buffer` maps hunk line numbers against the supplied buffer, so if
    // the worktree file differs from HEAD only in whitespace the hunk offsets
    // would be wrong when `ignore_whitespace` is set (libgit2 attributes hunks
    // by byte position in the HEAD blob, not the buffer). Feeding the HEAD blob
    // itself when the non-whitespace content is identical keeps the offsets
    // aligned while still honouring whitespace-agnostic attribution.
    let head_buffer = if config.ignore_whitespace {
        head_blob_contents(repository, &file.relative_path)?
            .filter(|head| lines_equivalent_ignoring_whitespace(head, buffer))
    } else {
        None
    };
    let buffer_blame = blame
        .blame_buffer(head_buffer.as_deref().unwrap_or(buffer))
        .map_err(|error| format!("git blame buffer failed: {error}"))?;

    for hunk in buffer_blame.iter() {
        attribute_hunk(&hunk, &lines, config, &mut counts);
    }

    Ok(counts)
}

fn line_authors_at(
    repository: &Repository,
    path: &std::path::Path,
    buffer: &[u8],
    newest: Option<Oid>,
    line_count: usize,
    config: &Config,
) -> Result<Vec<Option<LineAuthor>>, String> {
    let mut options = BlameOptions::new();
    options.ignore_whitespace(config.ignore_whitespace);
    options.use_mailmap(true);
    if let Some(commit) = newest {
        options.newest_commit(commit);
    }

    let blame = repository
        .blame_file(path, Some(&mut options))
        .map_err(|error| format!("git blame failed for parent: {error}"))?;
    let buffer_blame = blame
        .blame_buffer(buffer)
        .map_err(|error| format!("git blame buffer failed for parent: {error}"))?;

    Ok(line_authors(&buffer_blame, line_count, config))
}

#[derive(Clone)]
struct LineAuthor {
    author: String,
    committed: bool,
}

fn line_authors(blame: &Blame<'_>, line_count: usize, config: &Config) -> Vec<Option<LineAuthor>> {
    let mut lines = vec![None; line_count];

    for hunk in blame.iter() {
        let Some(author) = hunk_author(&hunk, config) else {
            continue;
        };
        let start = hunk.final_start_line().saturating_sub(1);
        for offset in 0..hunk.lines_in_hunk() {
            if let Some(slot) = lines.get_mut(start + offset) {
                *slot = Some(author.clone());
            }
        }
    }

    lines
}

fn count_conflict_lines(
    lines: &[&[u8]],
    head_lines: &[Option<LineAuthor>],
    merge_lines: &[Option<LineAuthor>],
    config: &Config,
    counts: &mut HashMap<String, u64>,
) {
    for (index, line) in lines.iter().enumerate() {
        if config.ignore_blank && is_blank(line) {
            continue;
        }

        let head = head_lines.get(index).and_then(Option::as_ref);
        let merge = merge_lines.get(index).and_then(Option::as_ref);
        let committed = head
            .filter(|author| author.committed)
            .or_else(|| merge.filter(|author| author.committed));
        let selected = committed.or(head).or(merge);

        if config.committed_only && !selected.is_some_and(|author| author.committed) {
            continue;
        }

        let author = selected
            .map(|author| author.author.clone())
            .unwrap_or_else(|| uncommitted_author(config).to_owned());
        *counts.entry(author).or_default() += 1;
    }
}

fn merge_head_oid(repository: &Repository) -> Option<Oid> {
    let contents = fs::read_to_string(repository.path().join("MERGE_HEAD")).ok()?;
    Oid::from_str(contents.lines().next()?.trim()).ok()
}

fn uncommitted_author(config: &Config) -> &'static str {
    if config.by_email {
        "not.committed.yet"
    } else {
        "Not Committed Yet"
    }
}

fn hunk_author(hunk: &git2::BlameHunk<'_>, config: &Config) -> Option<LineAuthor> {
    let uncommitted = hunk.final_commit_id() == Oid::ZERO_SHA1;
    let author = if uncommitted {
        uncommitted_author(config).to_owned()
    } else {
        let signature = hunk.final_signature()?;
        if config.by_email {
            String::from_utf8_lossy(signature.email_bytes())
                .trim()
                .to_lowercase()
        } else {
            String::from_utf8_lossy(signature.name_bytes())
                .trim()
                .to_owned()
        }
    };

    if author.is_empty() {
        return None;
    }

    Some(LineAuthor {
        author,
        committed: !uncommitted,
    })
}

fn split_lines(contents: &[u8]) -> Vec<&[u8]> {
    let mut lines: Vec<&[u8]> = contents.split(|byte| *byte == b'\n').collect();
    if contents.ends_with(b"\n") {
        lines.pop();
    }
    lines
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

fn commit_has_path(
    repository: &Repository,
    commit: Oid,
    path: &std::path::Path,
) -> Result<bool, String> {
    let commit = repository
        .find_commit(commit)
        .map_err(|error| format!("cannot read merge parent: {error}"))?;
    let tree = commit
        .tree()
        .map_err(|error| format!("cannot read merge parent tree: {error}"))?;

    match tree.get_path(path) {
        Ok(_) => Ok(true),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(false),
        Err(error) => Err(format!("cannot read merge parent tree: {error}")),
    }
}

fn head_blob_contents(
    repository: &Repository,
    path: &std::path::Path,
) -> Result<Option<Vec<u8>>, String> {
    let head = match repository.head() {
        Ok(head) => head,
        Err(error) if matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
            return Ok(None);
        }
        Err(error) => return Err(format!("cannot resolve HEAD: {error}")),
    };
    let tree = head
        .peel_to_tree()
        .map_err(|error| format!("cannot read HEAD tree: {error}"))?;

    match tree.get_path(path) {
        Ok(entry) => {
            let object = entry
                .to_object(repository)
                .map_err(|error| format!("cannot read HEAD blob: {error}"))?;
            Ok(object.as_blob().map(|blob| blob.content().to_vec()))
        }
        Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
        Err(error) => Err(format!("cannot read HEAD tree: {error}")),
    }
}

fn lines_equivalent_ignoring_whitespace(left: &[u8], right: &[u8]) -> bool {
    let left = split_lines(left);
    let right = split_lines(right);
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.iter()
                .filter(|byte| !byte.is_ascii_whitespace())
                .eq(right.iter().filter(|byte| !byte.is_ascii_whitespace()))
        })
}

fn path_in_head(repository: &Repository, path: &std::path::Path) -> Result<bool, String> {
    let head = match repository.head() {
        Ok(head) => head,
        Err(error) if matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
            return Ok(false);
        }
        Err(error) => return Err(format!("cannot resolve HEAD: {error}")),
    };

    let tree = head
        .peel_to_tree()
        .map_err(|error| format!("cannot read HEAD tree: {error}"))?;

    match tree.get_path(path) {
        Ok(_) => Ok(true),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(false),
        Err(error) => Err(format!("cannot read HEAD tree: {error}")),
    }
}

fn attribute_hunk(
    hunk: &git2::BlameHunk<'_>,
    lines: &[&[u8]],
    config: &Config,
    counts: &mut HashMap<String, u64>,
) {
    let Some(line_author) = hunk_author(hunk, config) else {
        return;
    };
    if config.committed_only && !line_author.committed {
        return;
    }

    let start = hunk.final_start_line().saturating_sub(1);
    let count = counts.entry(line_author.author).or_default();

    for offset in 0..hunk.lines_in_hunk() {
        let Some(line) = lines.get(start + offset) else {
            continue;
        };
        if config.ignore_blank && is_blank(line) {
            continue;
        }

        *count += 1;
    }
}

fn should_normalize_crlf(repository: &Repository) -> bool {
    let Ok(config) = repository.config() else {
        return false;
    };

    if config.get_bool("core.autocrlf").unwrap_or(false) {
        return true;
    }

    config
        .get_string("core.autocrlf")
        .map(|value| value.eq_ignore_ascii_case("input"))
        .unwrap_or(false)
}

fn normalize_crlf(contents: &[u8]) -> Vec<u8> {
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
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_crlf_only() {
        assert_eq!(normalize_crlf(b"a\r\nb\rc\n"), b"a\nb\rc\n");
        assert_eq!(normalize_crlf(b"no newline"), b"no newline");
    }

    #[test]
    fn compares_lines_ignoring_whitespace() {
        assert!(lines_equivalent_ignoring_whitespace(
            b"a b\nc\n",
            b"a  b\n c \n"
        ));
        assert!(!lines_equivalent_ignoring_whitespace(b"a b\nc\n", b"a b\n"));
        assert!(!lines_equivalent_ignoring_whitespace(
            b"a b\nc\n",
            b"a x\nc\n"
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
