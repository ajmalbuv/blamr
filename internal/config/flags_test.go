package config

import (
	"bytes"
	"flag"
	"reflect"
	"testing"
)

func TestParseDefaults(t *testing.T) {
	var buf bytes.Buffer
	cfg, err := Parse([]string{}, 4, &buf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if cfg.Workers != 4 {
		t.Errorf("expected 4 workers, got %d", cfg.Workers)
	}
	if cfg.ShowVersion {
		t.Errorf("expected ShowVersion to be false")
	}
	if cfg.JSONOutput {
		t.Errorf("expected JSONOutput to be false")
	}
	if cfg.ByEmail {
		t.Errorf("expected ByEmail to be false")
	}
	if cfg.IgnoreBlank {
		t.Errorf("expected IgnoreBlank to be false")
	}
	if len(cfg.Extensions) != 0 {
		t.Errorf("expected empty extensions, got %v", cfg.Extensions)
	}
	if len(cfg.Paths) != 0 {
		t.Errorf("expected empty paths, got %v", cfg.Paths)
	}
}

func TestParseExtensions(t *testing.T) {
	var buf bytes.Buffer
	cfg, err := Parse([]string{"-ext", "go, .TS, typ , "}, 2, &buf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	want := []string{".go", ".ts", ".typ"}
	if !reflect.DeepEqual(cfg.Extensions, want) {
		t.Errorf("got extensions %v, want %v", cfg.Extensions, want)
	}
}

func TestParseFlags(t *testing.T) {
	var buf bytes.Buffer
	cfg, err := Parse([]string{"-j", "8", "-json", "-email", "-ignore-blank", "-committed-only", "src", "pkg"}, 4, &buf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if cfg.Workers != 8 {
		t.Errorf("got workers %d, want 8", cfg.Workers)
	}
	if !cfg.JSONOutput {
		t.Errorf("expected JSONOutput to be true")
	}
	if !cfg.ByEmail {
		t.Errorf("expected ByEmail to be true")
	}
	if !cfg.IgnoreBlank {
		t.Errorf("expected IgnoreBlank to be true")
	}
	if !cfg.CommittedOnly {
		t.Errorf("expected CommittedOnly to be true")
	}
	wantPaths := []string{"src", "pkg"}
	if !reflect.DeepEqual(cfg.Paths, wantPaths) {
		t.Errorf("got paths %v, want %v", cfg.Paths, wantPaths)
	}
}

func TestParseFlagsAfterPaths(t *testing.T) {
	var buf bytes.Buffer
	cfg, err := Parse([]string{"src/auth", "-json", "-ext", ".go", "pkg/api"}, 4, &buf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !cfg.JSONOutput {
		t.Errorf("expected JSONOutput to be true")
	}
	wantExts := []string{".go"}
	if !reflect.DeepEqual(cfg.Extensions, wantExts) {
		t.Errorf("got extensions %v, want %v", cfg.Extensions, wantExts)
	}
	wantPaths := []string{"src/auth", "pkg/api"}
	if !reflect.DeepEqual(cfg.Paths, wantPaths) {
		t.Errorf("got paths %v, want %v", cfg.Paths, wantPaths)
	}
}

func TestParseVersion(t *testing.T) {
	var buf bytes.Buffer
	cfg, err := Parse([]string{"-v"}, 4, &buf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !cfg.ShowVersion {
		t.Errorf("expected ShowVersion true for -v")
	}

	cfg, err = Parse([]string{"-version"}, 4, &buf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !cfg.ShowVersion {
		t.Errorf("expected ShowVersion true for -version")
	}
}

func TestParseHelp(t *testing.T) {
	var buf bytes.Buffer
	_, err := Parse([]string{"-h"}, 4, &buf)
	if err != flag.ErrHelp {
		t.Errorf("expected flag.ErrHelp, got %v", err)
	}
	if !bytes.Contains(buf.Bytes(), []byte("Usage:")) {
		t.Errorf("expected usage output in buffer, got: %s", buf.String())
	}
}
