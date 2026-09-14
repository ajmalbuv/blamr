package filter

import (
	"os"
	"path/filepath"
	"testing"
)

func TestIsBinaryExt(t *testing.T) {
	binaryCases := []string{
		"test.png", "img.JPG", "archive.tar.gz", "app.exe",
		"lib.dll", "font.woff2", "doc.pdf", "data.sqlite", "vector.svg",
	}
	for _, p := range binaryCases {
		if !IsBinaryExt(p) {
			t.Errorf("expected %s to be recognized as binary extension", p)
		}
	}

	textCases := []string{
		"main.go", "script.py", "Cargo.toml", "README.md",
		"index.html", "style.css", "data.json", "query.sql",
	}
	for _, p := range textCases {
		if IsBinaryExt(p) {
			t.Errorf("expected %s to NOT be recognized as binary extension", p)
		}
	}
}

func TestIsBinaryData(t *testing.T) {
	tests := []struct {
		name     string
		data     []byte
		isBinary bool
	}{
		{
			name:     "empty data",
			data:     []byte{},
			isBinary: false,
		},
		{
			name:     "plain ascii",
			data:     []byte("hello world\npackage main\n"),
			isBinary: false,
		},
		{
			name:     "utf-8 with multibyte runes",
			data:     []byte("こんにちは世界 🚀 blamr line counter\n"),
			isBinary: false,
		},
		{
			name:     "null byte embedded in text",
			data:     []byte("hello\x00world"),
			isBinary: true,
		},
		{
			name:     "utf-16 le with bom",
			data:     []byte{0xFF, 0xFE, 'h', 0, 'i', 0},
			isBinary: false,
		},
		{
			name:     "utf-16 be with bom",
			data:     []byte{0xFE, 0xFF, 0, 'h', 0, 'i'},
			isBinary: false,
		},
		{
			name:     "git lfs pointer",
			data:     []byte("version https://git-lfs.github.com/spec/v1\noid sha256:4d7a..."),
			isBinary: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := IsBinaryData(tt.data)
			if got != tt.isBinary {
				t.Errorf("IsBinaryData() = %v, want %v", got, tt.isBinary)
			}
		})
	}
}

func TestIsGeneratedFile(t *testing.T) {
	generatedCases := []string{
		"package-lock.json", "yarn.lock", "pnpm-lock.yaml",
		"cargo.lock", "go.sum", "composer.lock", "bun.lock", "bun.lockb",
	}
	for _, f := range generatedCases {
		if !IsGeneratedFile(f) {
			t.Errorf("expected %s to be recognized as generated file", f)
		}
	}

	nonGenerated := []string{"package.json", "Cargo.toml", "go.mod", "main.go"}
	for _, f := range nonGenerated {
		if IsGeneratedFile(f) {
			t.Errorf("expected %s to NOT be recognized as generated file", f)
		}
	}
}

func TestIsMinifiedOrMap(t *testing.T) {
	minifiedCases := []string{"app.min.js", "style.min.css", "main.js.map"}
	for _, f := range minifiedCases {
		if !IsMinifiedOrMap(f) {
			t.Errorf("expected %s to be detected as minified/map", f)
		}
	}

	normalCases := []string{"app.js", "style.css", "main.go"}
	for _, f := range normalCases {
		if IsMinifiedOrMap(f) {
			t.Errorf("expected %s to NOT be detected as minified/map", f)
		}
	}
}

func TestMatchesExtensions(t *testing.T) {
	exts := []string{".go", ".ts"}
	if !MatchesExtensions("main.go", exts) {
		t.Errorf("expected main.go to match [.go, .ts]")
	}
	if !MatchesExtensions("APP.TS", exts) {
		t.Errorf("expected APP.TS to match [.go, .ts]")
	}
	if MatchesExtensions("style.css", exts) {
		t.Errorf("expected style.css to not match [.go, .ts]")
	}
	if !MatchesExtensions("anything.txt", nil) {
		t.Errorf("expected empty exts to match anything")
	}
}

func TestShouldSkip(t *testing.T) {
	tempDir := t.TempDir()

	textFile := filepath.Join(tempDir, "sample.txt")
	if err := os.WriteFile(textFile, []byte("some regular text\n"), 0644); err != nil {
		t.Fatal(err)
	}
	if ShouldSkip(textFile) {
		t.Errorf("expected ShouldSkip(sample.txt) to be false")
	}

	binFile := filepath.Join(tempDir, "sample.bin")
	if err := os.WriteFile(binFile, []byte("data\x00more"), 0644); err != nil {
		t.Fatal(err)
	}
	if !ShouldSkip(binFile) {
		t.Errorf("expected ShouldSkip(sample.bin) to be true")
	}

	lockFile := filepath.Join(tempDir, "yarn.lock")
	if err := os.WriteFile(lockFile, []byte("lockfile content"), 0644); err != nil {
		t.Fatal(err)
	}
	if !ShouldSkip(lockFile) {
		t.Errorf("expected ShouldSkip(yarn.lock) to be true")
	}
}
