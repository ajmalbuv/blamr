use std::fs::File;
use std::io::Read;
use std::path::Path;

const GENERATED_FILES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "cargo.lock",
    "go.sum",
    "composer.lock",
    "poetry.lock",
    "gemfile.lock",
    "mix.lock",
    "flake.lock",
    "pipfile.lock",
    "pubspec.lock",
    "bun.lock",
    "bun.lockb",
];

const BINARY_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".webp", ".bmp", ".tiff", ".psd", ".raw", ".svg",
    ".ai", ".eps", ".mp3", ".mp4", ".wav", ".ogg", ".flac", ".avi", ".mov", ".mkv", ".webm",
    ".m4a", ".zip", ".tar", ".gz", ".bz2", ".xz", ".7z", ".rar", ".zst", ".tgz", ".exe", ".dll",
    ".so", ".dylib", ".bin", ".wasm", ".o", ".a", ".lib", ".obj", ".pyc", ".pyo", ".class", ".jar",
    ".ttf", ".woff", ".woff2", ".eot", ".otf", ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt",
    ".pptx", ".db", ".sqlite", ".sqlite3",
];

const GIT_LFS_HEADER: &[u8] = b"version https://git-lfs.github.com/spec/v1";

pub fn matches_extensions(path: &str, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }
    let lower = path.to_lowercase();
    extensions
        .iter()
        .any(|extension| lower.ends_with(extension))
}

pub fn is_generated_file(path: &str) -> bool {
    let name = base_name(path).to_lowercase();
    GENERATED_FILES.contains(&name.as_str())
}

pub fn is_minified_or_map(path: &str) -> bool {
    let name = base_name(path).to_lowercase();
    name.ends_with(".min.js") || name.ends_with(".min.css") || name.ends_with(".map")
}

pub fn is_binary_extension(path: &str) -> bool {
    let extension = git_extension(path);
    BINARY_EXTENSIONS.contains(&extension.as_str())
}

pub fn is_binary_data(data: &[u8]) -> bool {
    if data.len() >= 2
        && ((data[0] == 0xFF && data[1] == 0xFE) || (data[0] == 0xFE && data[1] == 0xFF))
    {
        return false;
    }

    if data.starts_with(GIT_LFS_HEADER) {
        return true;
    }

    data.contains(&0)
}

pub fn should_skip(fs_path: &Path, git_path: &str) -> bool {
    if is_generated_file(git_path) || is_minified_or_map(git_path) || is_binary_extension(git_path)
    {
        return true;
    }

    let Ok(file) = File::open(fs_path) else {
        return false;
    };

    let mut buffer = [0_u8; 8192];
    let mut read = 0;
    let mut file = file;
    while read < buffer.len() {
        match file.read(&mut buffer[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => return false,
        }
    }

    is_binary_data(&buffer[..read])
}

fn base_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

// Use Git path semantics for extensions: the extension includes the final
// dot, even for dotfiles such as `.png`.
fn git_extension(path: &str) -> String {
    let name = base_name(path);
    match name.rfind('.') {
        Some(index) => name[index..].to_lowercase(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn binary_extensions() {
        for path in [
            "test.png",
            "img.JPG",
            "archive.tar.gz",
            "app.exe",
            "lib.dll",
            "font.woff2",
            "doc.pdf",
            "data.sqlite",
            "vector.svg",
            ".png",
        ] {
            assert!(is_binary_extension(path), "{path}");
        }

        for path in [
            "main.rs",
            "script.py",
            "Cargo.toml",
            "README.md",
            "index.html",
            "style.css",
            "data.json",
            "query.sql",
        ] {
            assert!(!is_binary_extension(path), "{path}");
        }
    }

    #[test]
    fn binary_data_rules() {
        assert!(!is_binary_data(b""));
        assert!(!is_binary_data(b"hello world\npackage main\n"));
        assert!(!is_binary_data(
            "こんにちは世界 🚀 blamr line counter\n".as_bytes()
        ));
        assert!(is_binary_data(b"hello\0world"));
        assert!(!is_binary_data(&[0xFF, 0xFE, b'h', 0, b'i', 0]));
        assert!(!is_binary_data(&[0xFE, 0xFF, 0, b'h', 0, b'i']));
        assert!(is_binary_data(
            b"version https://git-lfs.github.com/spec/v1\noid sha256:4d7a..."
        ));
    }

    #[test]
    fn generated_names() {
        for path in [
            "package-lock.json",
            "yarn.lock",
            "pnpm-lock.yaml",
            "cargo.lock",
            "go.sum",
            "composer.lock",
            "bun.lock",
            "bun.lockb",
        ] {
            assert!(is_generated_file(path), "{path}");
        }

        for path in ["package.json", "Cargo.toml", "index.html", "main.rs"] {
            assert!(!is_generated_file(path), "{path}");
        }
    }

    #[test]
    fn minified_names() {
        for path in ["app.min.js", "style.min.css", "main.js.map"] {
            assert!(is_minified_or_map(path), "{path}");
        }
        for path in ["app.js", "style.css", "main.rs"] {
            assert!(!is_minified_or_map(path), "{path}");
        }
    }

    #[test]
    fn extension_matching() {
        let extensions = vec![".rs".to_owned(), ".ts".to_owned()];
        assert!(matches_extensions("main.rs", &extensions));
        assert!(matches_extensions("APP.TS", &extensions));
        assert!(!matches_extensions("style.css", &extensions));
        assert!(matches_extensions("anything.txt", &[]));
    }

    #[test]
    fn skip_rules() {
        let dir = tempfile::tempdir().unwrap();
        let text_file = write_file(&dir.path().join("sample.txt"), b"some regular text\n");
        assert!(!should_skip(&text_file, "sample.txt"));

        let binary_file = write_file(&dir.path().join("sample.bin"), b"data\0more");
        assert!(should_skip(&binary_file, "sample.bin"));

        let lock_file = write_file(&dir.path().join("yarn.lock"), b"lockfile content");
        assert!(should_skip(&lock_file, "yarn.lock"));
    }

    fn write_file(path: &std::path::Path, contents: &[u8]) -> PathBuf {
        fs::write(path, contents).unwrap();
        path.to_path_buf()
    }
}
