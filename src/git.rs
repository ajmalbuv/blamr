use crate::config::Config;
use crate::filter;
use git2::Repository;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct TrackedPath {
    pub display_path: String,
    pub fs_path: PathBuf,
    pub relative_path: PathBuf,
    pub conflicted: bool,
}

impl TrackedPath {
    pub fn read_worktree(&self) -> std::io::Result<Vec<u8>> {
        if let Ok(metadata) = fs::symlink_metadata(&self.fs_path)
            && metadata.file_type().is_symlink()
        {
            return fs::read_link(&self.fs_path).map(|target| path_to_git_bytes(&target));
        }

        fs::read(&self.fs_path)
    }
}

pub fn open_repository() -> Result<Repository, String> {
    Repository::open_from_env()
        .map_err(|error| format!("not a git repo (or repository discovery failed): {error}"))
}

pub fn collect_tracked(config: &Config) -> Result<Vec<TrackedPath>, String> {
    let repo = open_repository()?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| "not a git repo (or repository has no worktree)".to_owned())?
        .to_path_buf();
    let cwd = std::env::current_dir().map_err(|error| format!("cannot determine CWD: {error}"))?;

    let workdir = canonicalize_or(workdir);
    let cwd = canonicalize_or(cwd);
    let cwd_prefix = cwd.strip_prefix(&workdir).ok().map(path_to_git_string);
    let explicit = explicit_paths(&config.paths, &cwd, &workdir)?;

    let index = repo
        .index()
        .map_err(|error| format!("cannot open repository index: {error}"))?;

    let mut tracked = Vec::new();

    for entry in index.iter() {
        if entry.path.is_empty() {
            continue;
        }

        let git_path = String::from_utf8_lossy(&entry.path).into_owned();
        if !explicit.is_empty() && !explicit.iter().any(|base| path_matches(&git_path, base)) {
            continue;
        }
        if explicit.is_empty()
            && let Some(prefix) = cwd_prefix.as_deref()
            && !prefix.is_empty()
            && !path_matches(&git_path, prefix)
        {
            continue;
        }

        let relative_path = git_path_to_native(&entry.path);
        let fs_path = workdir.join(&relative_path);

        if config.extensions.is_empty() {
            if filter::should_skip(&fs_path, &git_path) {
                continue;
            }
        } else if !filter::matches_extensions(&git_path, &config.extensions) {
            continue;
        }

        let display_path = display_path(&git_path, cwd_prefix.as_deref());
        tracked.push(TrackedPath {
            display_path,
            fs_path,
            relative_path,
            conflicted: entry.flags & 0x3000 != 0,
        });
    }

    Ok(tracked)
}

fn explicit_paths(paths: &[OsString], cwd: &Path, workdir: &Path) -> Result<Vec<String>, String> {
    let mut bases = Vec::with_capacity(paths.len());

    for value in paths {
        let path = Path::new(value);
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        let absolute = canonicalize_or(absolute);
        let absolute = normalize_path(&absolute);
        let Some(relative) = relative_to_workdir(&absolute, workdir) else {
            return Err(format!(
                "path is outside repository: {}",
                path.to_string_lossy()
            ));
        };
        bases.push(path_to_git_string(&relative));
    }

    Ok(bases)
}

fn relative_to_workdir(path: &Path, workdir: &Path) -> Option<PathBuf> {
    if let Ok(relative) = path.strip_prefix(workdir) {
        return Some(relative.to_path_buf());
    }

    #[cfg(windows)]
    {
        let path = strip_verbatim(path);
        let workdir = strip_verbatim(workdir);
        if let Ok(relative) = path.strip_prefix(&workdir) {
            return Some(relative.to_path_buf());
        }
        return strip_prefix_case_insensitive(&path, &workdir);
    }

    #[allow(unreachable_code)]
    None
}

#[cfg(windows)]
fn strip_prefix_case_insensitive(path: &Path, base: &Path) -> Option<PathBuf> {
    let path_components: Vec<Component<'_>> = path.components().collect();
    let base_components: Vec<Component<'_>> = base.components().collect();
    if path_components.len() < base_components.len() {
        return None;
    }

    for (path_component, base_component) in path_components.iter().zip(&base_components) {
        let path_text = path_component.as_os_str().to_string_lossy();
        let base_text = base_component.as_os_str().to_string_lossy();
        if !path_text.eq_ignore_ascii_case(&base_text) {
            return None;
        }
    }

    let mut relative = PathBuf::new();
    for component in &path_components[base_components.len()..] {
        relative.push(component.as_os_str());
    }
    Some(relative)
}

#[cfg(windows)]
fn strip_verbatim(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

fn path_matches(git_path: &str, base: &str) -> bool {
    if base.is_empty() {
        return true;
    }
    git_path == base
        || git_path
            .strip_prefix(base)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn display_path(git_path: &str, cwd_prefix: Option<&str>) -> String {
    let Some(prefix) = cwd_prefix else {
        return git_path.to_owned();
    };
    if prefix.is_empty() {
        return git_path.to_owned();
    }

    let cwd_parts: Vec<&str> = prefix.split('/').filter(|part| !part.is_empty()).collect();
    let file_parts: Vec<&str> = git_path.split('/').collect();
    let common = cwd_parts
        .iter()
        .zip(file_parts.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let mut parts = Vec::with_capacity(cwd_parts.len() - common + file_parts.len() - common);
    for _ in common..cwd_parts.len() {
        parts.push("..".to_owned());
    }
    for part in &file_parts[common..] {
        parts.push((*part).to_owned());
    }

    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

fn git_path_to_native(raw: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(OsStr::from_bytes(raw))
    }

    #[cfg(not(unix))]
    {
        let mut path = PathBuf::new();
        for part in String::from_utf8_lossy(raw).split('/') {
            path.push(part);
        }
        path
    }
}

fn path_to_git_string(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn path_to_git_bytes(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }

    #[cfg(not(unix))]
    {
        path.to_string_lossy().replace('\\', "/").into_bytes()
    }
}

fn canonicalize_or(path: PathBuf) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(&path) {
        return canonical;
    }

    let Some(name) = path.file_name() else {
        return path;
    };
    let Some(parent) = path.parent() else {
        return path;
    };
    match fs::canonicalize(parent) {
        Ok(canonical_parent) => canonical_parent.join(name),
        Err(_) => path,
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => result.push(prefix.as_os_str()),
            Component::RootDir => result.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let can_pop = result
                    .components()
                    .next_back()
                    .is_some_and(|last| matches!(last, Component::Normal(_)));
                if can_pop {
                    result.pop();
                } else if !result.has_root() {
                    result.push("..");
                }
            }
            Component::Normal(value) => result.push(value),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_matching_handles_files_and_directories() {
        assert!(path_matches("src/main.rs", "src"));
        assert!(path_matches("src/main.rs", "src/main.rs"));
        assert!(path_matches("src/main.rs", ""));
        assert!(!path_matches("src/main.rs", "src/main"));
        assert!(!path_matches("src/main.rs", "other"));
    }

    #[test]
    fn display_paths_are_cwd_relative() {
        assert_eq!(display_path("internal/app.rs", Some("internal")), "app.rs");
        assert_eq!(display_path("README.md", Some("internal")), "../README.md");
        assert_eq!(display_path("README.md", Some("")), "README.md");
        assert_eq!(display_path("README.md", None), "README.md");
    }

    #[test]
    fn normalizes_parent_components() {
        let base = std::env::temp_dir();
        assert_eq!(
            normalize_path(&base.join("a").join("..").join("b")),
            base.join("b")
        );
    }

    #[cfg(windows)]
    #[test]
    fn absolute_paths_under_worktree_match_case_insensitively() {
        let workdir = Path::new(r"C:\Repo");
        let relative = relative_to_workdir(Path::new(r"c:\repo\src\main.rs"), workdir);
        assert_eq!(relative.as_deref(), Some(Path::new(r"src\main.rs")));
        assert!(relative_to_workdir(Path::new(r"C:\Repo2\src\main.rs"), workdir).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn absolute_paths_under_worktree_match() {
        let workdir = Path::new("/repo");
        assert_eq!(
            relative_to_workdir(Path::new("/repo/src/main.rs"), workdir).as_deref(),
            Some(Path::new("src/main.rs"))
        );
        assert!(relative_to_workdir(Path::new("/repo2/src/main.rs"), workdir).is_none());
    }
}
