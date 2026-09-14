package output

import (
	"encoding/json"
	"io"

	"github.com/ajmalbuv/blamr/internal/blame"
)

// PrintJSON serializes the blame statistics as indented JSON into w.
func PrintJSON(w io.Writer, stats *blame.Stats) error {
	enc := json.NewEncoder(w)
	enc.SetIndent("", "  ")
	return enc.Encode(stats)
}
