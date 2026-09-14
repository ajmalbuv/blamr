package filter

import (
	"os"
	"path/filepath"
)

// ShouldSkip inspects the file path and content to determine if it should be omitted.
func ShouldSkip(path string) bool {
	if IsGeneratedFile(path) {
		return true
	}
	if IsMinifiedOrMap(path) {
		return true
	}
	if IsBinaryExt(path) {
		return true
	}

	// Content inspection: check the first 8KB.
	// Uses stack buffer inside IsBinaryReader to avoid heap allocations.
	f, err := os.Open(filepath.FromSlash(path))
	if err != nil {
		return false // If deleted or inaccessible locally, let git blame resolve it
	}
	defer func() { _ = f.Close() }()

	isBinary, err := IsBinaryReader(f)
	if err != nil {
		return false
	}
	return isBinary
}
