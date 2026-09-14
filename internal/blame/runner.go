package blame

import (
	"fmt"
	"math"
	"os"
	"os/exec"
	"sort"
	"strings"
	"sync"

	"github.com/ajmalbuv/blamr/internal/config"
	"github.com/ajmalbuv/blamr/internal/filter"
)

// AuthorStat represents the aggregated line count and percentage for a single author.
type AuthorStat struct {
	Author     string  `json:"author"`
	Lines      int     `json:"lines"`
	Percentage float64 `json:"percentage"`
}

// Stats contains the aggregated results across all tracked files.
type Stats struct {
	TotalFiles int          `json:"total_files"`
	TotalLines int          `json:"total_lines"`
	Authors    []AuthorStat `json:"authors"`
}

// ProgressFunc is invoked as each file completes blame processing.
type ProgressFunc func(done, total int, currentFile string)

// CollectFiles queries git for tracked files and applies filtering rules.
func CollectFiles(cfg *config.Config) ([]string, error) {
	if _, err := exec.LookPath("git"); err != nil {
		return nil, fmt.Errorf("git not found on PATH: %w", err)
	}

	args := []string{"ls-files"}
	if len(cfg.Paths) > 0 {
		args = append(args, "--")
		args = append(args, cfg.Paths...)
	}

	out, err := exec.Command("git", args...).Output()
	if err != nil {
		return nil, fmt.Errorf("not a git repo (or git ls-files failed): %w", err)
	}

	var files []string
	for _, f := range strings.Split(strings.TrimSpace(string(out)), "\n") {
		f = strings.TrimSpace(f)
		if f == "" {
			continue
		}

		if len(cfg.Extensions) > 0 {
			if filter.MatchesExtensions(f, cfg.Extensions) {
				files = append(files, f)
			}
		} else {
			if !filter.ShouldSkip(f) {
				files = append(files, f)
			}
		}
	}

	return files, nil
}

// BlameFile runs git blame on a single file and parses line-porcelain output.
func BlameFile(path string, byEmail, ignoreBlank, committedOnly bool) (map[string]int, error) {
	// -w ignores whitespace churn (reformatting, tabs vs spaces)
	cmd := exec.Command("git", "blame", "--line-porcelain", "-w", "--", path)
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, fmt.Errorf("blame %s: %w", path, err)
	}
	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("blame %s: %w", path, err)
	}

	authors := ParsePorcelain(stdout, byEmail, ignoreBlank, committedOnly)
	if err := cmd.Wait(); err != nil {
		return nil, fmt.Errorf("blame %s: %w", path, err)
	}
	return authors, nil
}

// Run executes the blame pipeline across files using a worker pool.
func Run(cfg *config.Config, onProgress ProgressFunc) (*Stats, error) {
	files, err := CollectFiles(cfg)
	if err != nil {
		return nil, err
	}

	total := len(files)
	if total == 0 {
		return &Stats{
			TotalFiles: 0,
			TotalLines: 0,
			Authors:    []AuthorStat{},
		}, nil
	}

	type result struct {
		file    string
		authors map[string]int
		err     error
	}

	jobs := make(chan string, cfg.Workers*2)
	results := make(chan result, cfg.Workers*2)
	var wg sync.WaitGroup

	for i := 0; i < cfg.Workers; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for f := range jobs {
				authors, err := BlameFile(f, cfg.ByEmail, cfg.IgnoreBlank, cfg.CommittedOnly)
				results <- result{file: f, authors: authors, err: err}
			}
		}()
	}

	go func() {
		for _, f := range files {
			jobs <- f
		}
		close(jobs)
	}()

	go func() {
		wg.Wait()
		close(results)
	}()

	authorLines := make(map[string]int)
	done := 0
	succeeded := 0
	for r := range results {
		done++
		if onProgress != nil {
			onProgress(done, total, r.file)
		}
		if r.err != nil {
			fmt.Fprintf(os.Stderr, "warning: %v\n", r.err)
			continue
		}
		succeeded++
		for author, n := range r.authors {
			authorLines[author] += n
		}
	}

	totalLines := 0
	for _, n := range authorLines {
		totalLines += n
	}

	authors := make([]AuthorStat, 0, len(authorLines))
	for a, c := range authorLines {
		pct := 0.0
		if totalLines > 0 {
			pct = math.Round((float64(c)/float64(totalLines)*100)*100) / 100
		}
		authors = append(authors, AuthorStat{
			Author:     a,
			Lines:      c,
			Percentage: pct,
		})
	}

	sort.Slice(authors, func(i, j int) bool {
		if authors[i].Lines != authors[j].Lines {
			return authors[i].Lines > authors[j].Lines
		}
		return authors[i].Author < authors[j].Author
	})

	return &Stats{
		TotalFiles: succeeded,
		TotalLines: totalLines,
		Authors:    authors,
	}, nil
}
