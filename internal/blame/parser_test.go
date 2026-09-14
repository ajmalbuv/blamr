package blame

import (
	"strings"
	"testing"
)

const samplePorcelain = `d2a3f01b4c 1 1 3
author Alice Smith
author-mail <Alice.Smith@example.com>
author-time 1672531199
committer Alice Smith
filename main.go
	package main
d2a3f01b4c 2 2
author Alice Smith
author-mail <Alice.Smith@example.com>
author-time 1672531199
committer Alice Smith
filename main.go
	
d2a3f01b4c 3 3
author Alice Smith
author-mail <Alice.Smith@example.com>
author-time 1672531199
committer Alice Smith
filename main.go
		func main() {}
e8c4b2a101 4 4 2
author Bob Jones
author-mail <bob@example.com>
author-time 1672532200
committer Bob Jones
filename main.go
	   
e8c4b2a101 5 5
author Bob Jones
author-mail <bob@example.com>
author-time 1672532200
committer Bob Jones
filename main.go
	// comment
`

func TestParsePorcelainByAuthor(t *testing.T) {
	counts := ParsePorcelain(strings.NewReader(samplePorcelain), false, false, false)

	if counts["Alice Smith"] != 3 {
		t.Errorf("got Alice Smith %d, want 3", counts["Alice Smith"])
	}
	if counts["Bob Jones"] != 2 {
		t.Errorf("got Bob Jones %d, want 2", counts["Bob Jones"])
	}
}

func TestParsePorcelainByEmail(t *testing.T) {
	counts := ParsePorcelain(strings.NewReader(samplePorcelain), true, false, false)

	if counts["alice.smith@example.com"] != 3 {
		t.Errorf("got alice.smith@example.com %d, want 3", counts["alice.smith@example.com"])
	}
	if counts["bob@example.com"] != 2 {
		t.Errorf("got bob@example.com %d, want 2", counts["bob@example.com"])
	}
}

func TestParsePorcelainIgnoreBlank(t *testing.T) {
	// In samplePorcelain:
	// Alice Smith has:
	//   line 1: "package main" (text)
	//   line 2: "" (empty)
	//   line 3: "\tfunc main() {}" (text)
	// => with ignoreBlank=true, Alice should have 2 lines.
	// Bob Jones has:
	//   line 4: "   " (spaces only)
	//   line 5: "// comment" (text)
	// => with ignoreBlank=true, Bob should have 1 line.
	counts := ParsePorcelain(strings.NewReader(samplePorcelain), false, true, false)

	if counts["Alice Smith"] != 2 {
		t.Errorf("got Alice Smith %d, want 2", counts["Alice Smith"])
	}
	if counts["Bob Jones"] != 1 {
		t.Errorf("got Bob Jones %d, want 1", counts["Bob Jones"])
	}
}

func TestParsePorcelainCommittedOnly(t *testing.T) {
	uncommittedSample := `0000000000 1 1 1
author Not Committed Yet
author-mail <not.committed.yet>
filename main.go
	new line
`
	counts := ParsePorcelain(strings.NewReader(uncommittedSample), false, false, true)
	if len(counts) != 0 {
		t.Errorf("expected 0 lines for uncommitted with committedOnly=true, got %v", counts)
	}

	countsEmail := ParsePorcelain(strings.NewReader(uncommittedSample), true, false, true)
	if len(countsEmail) != 0 {
		t.Errorf("expected 0 lines for uncommitted email with committedOnly=true, got %v", countsEmail)
	}
}
