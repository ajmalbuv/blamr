package blame

import (
	"bufio"
	"io"
	"strings"
)

// ParsePorcelain parses git blame --line-porcelain output from r.
// It attributes lines to authors (or emails if byEmail is true),
// skips blank lines if ignoreBlank is true, and excludes uncommitted
// working tree changes if committedOnly is true.
func ParsePorcelain(r io.Reader, byEmail, ignoreBlank, committedOnly bool) map[string]int {
	authors := make(map[string]int)
	reader := bufio.NewReader(r)

	var currentAuthor string

	for {
		line, err := reader.ReadString('\n')
		if len(line) > 0 {
			line = strings.TrimRight(line, "\r\n")

			if strings.HasPrefix(line, "author ") {
				if !byEmail {
					currentAuthor = strings.TrimSpace(line[len("author "):])
				}
			} else if strings.HasPrefix(line, "author-mail ") {
				if byEmail {
					mail := strings.TrimSpace(line[len("author-mail "):])
					mail = strings.TrimPrefix(mail, "<")
					mail = strings.TrimSuffix(mail, ">")
					currentAuthor = strings.ToLower(mail)
				}
			} else if strings.HasPrefix(line, "\t") {
				// \t denotes the actual source code line in porcelain format
				if ignoreBlank && strings.TrimSpace(line[1:]) == "" {
					// Blank or whitespace-only line skipped
				} else if committedOnly && (currentAuthor == "Not Committed Yet" || currentAuthor == "not.committed.yet") {
					// Uncommitted working tree line skipped
				} else if currentAuthor != "" {
					authors[currentAuthor]++
				}
			}
		}

		if err != nil {
			break
		}
	}

	return authors
}
