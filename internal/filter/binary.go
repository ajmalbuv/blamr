package filter

import (
	"bytes"
	"io"
	"path/filepath"
	"strings"
)

// binaryExts maps extensions known to be non-text.
// Skipped immediately without filesystem I/O.
// struct{} ensures zero heap allocation per entry.
var binaryExts = map[string]struct{}{
	// Images & Design
	".png": {}, ".jpg": {}, ".jpeg": {}, ".gif": {}, ".ico": {},
	".webp": {}, ".bmp": {}, ".tiff": {}, ".psd": {}, ".raw": {},
	".svg": {}, ".ai": {}, ".eps": {},
	// Video & Audio
	".mp3": {}, ".mp4": {}, ".wav": {}, ".ogg": {}, ".flac": {},
	".avi": {}, ".mov": {}, ".mkv": {}, ".webm": {}, ".m4a": {},
	// Archives & Compressed
	".zip": {}, ".tar": {}, ".gz": {}, ".bz2": {}, ".xz": {},
	".7z": {}, ".rar": {}, ".zst": {}, ".tgz": {},
	// Executables, Binaries & Libraries
	".exe": {}, ".dll": {}, ".so": {}, ".dylib": {}, ".bin": {},
	".wasm": {}, ".o": {}, ".a": {}, ".lib": {}, ".obj": {},
	// Bytecode & Runtime
	".pyc": {}, ".pyo": {}, ".class": {}, ".jar": {},
	// Fonts
	".ttf": {}, ".woff": {}, ".woff2": {}, ".eot": {}, ".otf": {},
	// Documents & Databases
	".pdf": {}, ".doc": {}, ".docx": {}, ".xls": {}, ".xlsx": {},
	".ppt": {}, ".pptx": {}, ".db": {}, ".sqlite": {}, ".sqlite3": {},
}

// gitLFSHeader is the spec prefix for Git LFS pointer files.
var gitLFSHeader = []byte("version https://git-lfs.github.com/spec/v1")

// IsBinaryExt returns true if the file extension is recognized as non-text.
func IsBinaryExt(path string) bool {
	ext := strings.ToLower(filepath.Ext(path))
	_, ok := binaryExts[ext]
	return ok
}

// IsBinaryData inspects up to 8KB of byte data to determine if it is binary
// or should be skipped (e.g. Git LFS pointers).
// Uses SIMD-accelerated bytes.IndexByte and checks for UTF-16 BOM.
func IsBinaryData(data []byte) bool {
	// UTF-16 BOM check (LE: FF FE, BE: FE FF). UTF-16 files contain null bytes but are text.
	if len(data) >= 2 {
		if (data[0] == 0xFF && data[1] == 0xFE) || (data[0] == 0xFE && data[1] == 0xFF) {
			return false
		}
	}

	// Git LFS pointer check
	if bytes.HasPrefix(data, gitLFSHeader) {
		return true
	}

	// Git/GNU diff standard: search for null byte (0x00).
	// bytes.IndexByte is SIMD-accelerated in the Go standard library.
	return bytes.IndexByte(data, 0) != -1
}

// IsBinaryReader reads up to 8KB from r using a stack-allocated buffer
// to determine whether the contents are binary.
func IsBinaryReader(r io.Reader) (bool, error) {
	var buf [8192]byte
	n, err := io.ReadFull(r, buf[:])
	if err != nil && err != io.EOF && err != io.ErrUnexpectedEOF {
		return false, err
	}
	return IsBinaryData(buf[:n]), nil
}
