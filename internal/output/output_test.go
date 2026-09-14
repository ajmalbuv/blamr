package output

import (
	"bytes"
	"encoding/json"
	"strings"
	"testing"

	"github.com/ajmalbuv/blamr/internal/blame"
)

func TestTruncate(t *testing.T) {
	if got := Truncate("hello", 10); got != "hello" {
		t.Errorf("got %q, want %q", got, "hello")
	}
	if got := Truncate("hello world", 5); got != "hello" {
		t.Errorf("got %q, want %q", got, "hello")
	}
	// Unicode multibyte test (emojis)
	if got := Truncate("🚀🚀🚀🚀🚀", 2); got != "🚀🚀" {
		t.Errorf("got %q, want %q", got, "🚀🚀")
	}
}

func TestPrintProgress(t *testing.T) {
	var buf bytes.Buffer

	// Piped / non-TTY should produce zero output
	PrintProgress(&buf, 1, 10, "main.go", false)
	if buf.Len() != 0 {
		t.Errorf("expected no output for non-TTY, got %q", buf.String())
	}

	// Interactive TTY should produce carriage return and clear-line sequence
	PrintProgress(&buf, 1, 10, "main.go", true)
	out := buf.String()
	if !strings.HasPrefix(out, "\r[1/10]") {
		t.Errorf("expected prefix \\r[1/10], got %q", out)
	}
	if !strings.HasSuffix(out, "\033[K") {
		t.Errorf("expected suffix \\033[K, got %q", out)
	}
}

func TestPrintTable(t *testing.T) {
	stats := &blame.Stats{
		TotalFiles: 2,
		TotalLines: 100,
		Authors: []blame.AuthorStat{
			{Author: "Alice", Lines: 70, Percentage: 70.0},
			{Author: "Bob", Lines: 30, Percentage: 30.0},
		},
	}

	var buf bytes.Buffer
	PrintTable(&buf, stats)
	out := buf.String()

	if !strings.Contains(out, "Author") || !strings.Contains(out, "Lines") || !strings.Contains(out, "%") {
		t.Errorf("table missing header: %s", out)
	}
	if !strings.Contains(out, "Alice") || !strings.Contains(out, "70") {
		t.Errorf("table missing Alice: %s", out)
	}
	if !strings.Contains(out, "Bob") || !strings.Contains(out, "30") {
		t.Errorf("table missing Bob: %s", out)
	}
	if !strings.Contains(out, "Total") || !strings.Contains(out, "100") {
		t.Errorf("table missing total row: %s", out)
	}
}

func TestPrintJSON(t *testing.T) {
	stats := &blame.Stats{
		TotalFiles: 2,
		TotalLines: 100,
		Authors: []blame.AuthorStat{
			{Author: "Alice", Lines: 70, Percentage: 70.0},
			{Author: "Bob", Lines: 30, Percentage: 30.0},
		},
	}

	var buf bytes.Buffer
	if err := PrintJSON(&buf, stats); err != nil {
		t.Fatalf("unexpected JSON encode error: %v", err)
	}

	var decoded map[string]interface{}
	if err := json.Unmarshal(buf.Bytes(), &decoded); err != nil {
		t.Fatalf("failed to parse generated JSON: %v", err)
	}

	if decoded["total_files"].(float64) != 2 {
		t.Errorf("got total_files %v, want 2", decoded["total_files"])
	}
	if decoded["total_lines"].(float64) != 100 {
		t.Errorf("got total_lines %v, want 100", decoded["total_lines"])
	}
	authors := decoded["authors"].([]interface{})
	if len(authors) != 2 {
		t.Errorf("got %d authors, want 2", len(authors))
	}
}
