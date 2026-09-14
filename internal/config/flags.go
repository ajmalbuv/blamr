package config

import (
	"flag"
	"fmt"
	"io"
	"strings"
)

// Config holds the validated runtime configuration for blamr.
type Config struct {
	Extensions    []string
	Workers       int
	JSONOutput    bool
	ShowVersion   bool
	ByEmail       bool
	IgnoreBlank   bool
	CommittedOnly bool
	Paths         []string
}

// Parse parses CLI arguments and returns a populated Config.
// An io.Writer is provided for writing usage/help messages.
func Parse(args []string, defaultWorkers int, out io.Writer) (*Config, error) {
	fs := flag.NewFlagSet("blamr", flag.ContinueOnError)
	fs.SetOutput(out)

	fs.Usage = func() {
		_, _ = fmt.Fprintf(out, "blamr: concurrent git-blame author attribution and line statistics\n\n")
		_, _ = fmt.Fprintf(out, "Usage:\n")
		_, _ = fmt.Fprintf(out, "  blamr [flags] [paths...]\n")
		_, _ = fmt.Fprintf(out, "  git blamr [flags] [paths...]\n\n")
		_, _ = fmt.Fprintf(out, "Flags:\n")
		fs.PrintDefaults()
	}

	extFlag := fs.String("ext", "", "comma-separated extensions to filter (e.g. .typ,.tex,.go). Defaults to all text files.")
	workers := fs.Int("j", defaultWorkers, "number of concurrent git-blame workers")
	versionFlag := fs.Bool("version", false, "print version information and exit")
	vFlag := fs.Bool("v", false, "print version information and exit (shorthand)")
	jsonFlag := fs.Bool("json", false, "output results as structured JSON")
	emailFlag := fs.Bool("email", false, "aggregate lines by author email instead of name")
	ignoreBlankFlag := fs.Bool("ignore-blank", false, "ignore empty and whitespace-only lines")
	committedOnlyFlag := fs.Bool("committed-only", false, "exclude uncommitted working tree changes from attribution")

	var flagArgs, posArgs []string
	seenSeparator := false
	for i := 0; i < len(args); i++ {
		arg := args[i]
		if arg == "--" {
			seenSeparator = true
			posArgs = append(posArgs, args[i+1:]...)
			break
		}
		if strings.HasPrefix(arg, "-") {
			flagArgs = append(flagArgs, arg)
			// Handle space-separated value flags (-ext .go or -j 4)
			if (arg == "-ext" || arg == "-j") && i+1 < len(args) && !strings.HasPrefix(args[i+1], "-") {
				i++
				flagArgs = append(flagArgs, args[i])
			}
		} else {
			posArgs = append(posArgs, arg)
		}
	}
	orderedArgs := flagArgs
	if seenSeparator {
		orderedArgs = append(orderedArgs, "--")
	}
	orderedArgs = append(orderedArgs, posArgs...)

	if err := fs.Parse(orderedArgs); err != nil {
		return nil, err
	}

	cfg := &Config{
		Workers:       *workers,
		ShowVersion:   *versionFlag || *vFlag,
		JSONOutput:    *jsonFlag,
		ByEmail:       *emailFlag,
		IgnoreBlank:   *ignoreBlankFlag,
		CommittedOnly: *committedOnlyFlag,
		Paths:         fs.Args(),
	}

	if cfg.Workers <= 0 {
		cfg.Workers = defaultWorkers
		if cfg.Workers <= 0 {
			cfg.Workers = 1
		}
	}

	if strings.TrimSpace(*extFlag) != "" {
		for _, e := range strings.Split(*extFlag, ",") {
			e = strings.TrimSpace(strings.ToLower(e))
			if e == "" {
				continue
			}
			if !strings.HasPrefix(e, ".") {
				e = "." + e
			}
			cfg.Extensions = append(cfg.Extensions, e)
		}
	}

	return cfg, nil
}
