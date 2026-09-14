package app

import (
	"flag"
	"fmt"
	"os"
	"runtime"
	"strings"

	"github.com/ajmalbuv/blamr/internal/blame"
	"github.com/ajmalbuv/blamr/internal/config"
	"github.com/ajmalbuv/blamr/internal/output"
)

// Run executes the blamr application lifecycle given the build version.
func Run(version string) {
	cfg, err := config.Parse(os.Args[1:], runtime.NumCPU(), os.Stderr)
	if err != nil {
		if err == flag.ErrHelp {
			os.Exit(0)
		}
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(2)
	}

	if cfg.ShowVersion {
		fmt.Printf("blamr version %s (%s/%s)\n", version, runtime.GOOS, runtime.GOARCH)
		return
	}

	isTTY := output.IsTerminal(os.Stdout)
	progressFn := func(done, total int, currentFile string) {
		if !cfg.JSONOutput {
			output.PrintProgress(os.Stdout, done, total, currentFile, isTTY)
		}
	}

	stats, err := blame.Run(cfg, progressFn)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}

	if stats.TotalFiles == 0 {
		if cfg.JSONOutput {
			_ = output.PrintJSON(os.Stdout, stats)
			return
		}
		if len(cfg.Extensions) > 0 {
			fmt.Printf("no matching files (%s) tracked in this repo\n", strings.Join(cfg.Extensions, ", "))
		} else {
			fmt.Println("no text files found tracked in this repo")
		}
		return
	}

	if cfg.JSONOutput {
		if err := output.PrintJSON(os.Stdout, stats); err != nil {
			fmt.Fprintf(os.Stderr, "error serializing JSON: %v\n", err)
			os.Exit(1)
		}
		return
	}

	output.PrintTable(os.Stdout, stats)
}
