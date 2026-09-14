package output

import (
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/ajmalbuv/blamr/internal/blame"
)

// IsTerminal checks if the given io.Writer is an interactive terminal (character device).
func IsTerminal(w io.Writer) bool {
	f, ok := w.(*os.File)
	if !ok {
		return false
	}
	stat, err := f.Stat()
	return err == nil && (stat.Mode()&os.ModeCharDevice) != 0
}

// Truncate ensures string s does not exceed length n runes.
func Truncate(s string, n int) string {
	runes := []rune(s)
	if len(runes) <= n {
		return s
	}
	return string(runes[:n])
}

// PrintProgress writes an interactive single-line progress update if isTTY is true.
// When output is redirected to a pipe or file, progress output is suppressed to avoid log clutter.
func PrintProgress(w io.Writer, done, total int, currentFile string, isTTY bool) {
	if !isTTY {
		return
	}
	// \033[K clears the remainder of the line to prevent trailing artifact characters
	_, _ = fmt.Fprintf(w, "\r[%d/%d] %-60s\033[K", done, total, Truncate(currentFile, 60))
}

// PrintTable renders the formatted line statistics table with Unicode bar charts.
func PrintTable(w io.Writer, stats *blame.Stats) {
	divider := strings.Repeat("─", 45)

	_, _ = fmt.Fprintf(w, "\n\n%s\n", divider)
	_, _ = fmt.Fprintf(w, "%-25s %8s %8s\n", "Author", "Lines", "%")
	_, _ = fmt.Fprintf(w, "%s\n", divider)

	for _, a := range stats.Authors {
		bar := strings.Repeat("█", int(a.Percentage/5))
		_, _ = fmt.Fprintf(w, "%-25s %8d %7.1f%%  %s\n", Truncate(a.Author, 25), a.Lines, a.Percentage, bar)
	}

	_, _ = fmt.Fprintf(w, "%s\n", divider)
	_, _ = fmt.Fprintf(w, "%-25s %8d\n", "Total", stats.TotalLines)
}
