package filter

import (
	"path/filepath"
	"strings"
)

// generatedFiles contains common machine-generated lockfiles that skew line statistics.
var generatedFiles = map[string]struct{}{
	"package-lock.json": {}, "yarn.lock": {}, "pnpm-lock.yaml": {},
	"cargo.lock": {}, "go.sum": {}, "composer.lock": {},
	"poetry.lock": {}, "gemfile.lock": {}, "mix.lock": {},
	"flake.lock": {}, "pipfile.lock": {}, "pubspec.lock": {},
	"bun.lock": {}, "bun.lockb": {},
}

// IsGeneratedFile returns true if the base filename is a known lockfile.
func IsGeneratedFile(path string) bool {
	name := strings.ToLower(filepath.Base(path))
	_, ok := generatedFiles[name]
	return ok
}

// IsMinifiedOrMap returns true if the file is a minified bundle or source map.
func IsMinifiedOrMap(path string) bool {
	name := strings.ToLower(filepath.Base(path))
	return strings.HasSuffix(name, ".min.js") ||
		strings.HasSuffix(name, ".min.css") ||
		strings.HasSuffix(name, ".map")
}

// MatchesExtensions returns true if the path ends with one of the provided extensions.
// If exts is empty, all files match (returns true).
func MatchesExtensions(path string, exts []string) bool {
	if len(exts) == 0 {
		return true
	}
	lower := strings.ToLower(path)
	for _, ext := range exts {
		if strings.HasSuffix(lower, ext) {
			return true
		}
	}
	return false
}
