set shell := ["sh", "-c"]
set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

binary := if os() == "windows" { "blamr.exe" } else { "blamr" }
git_binary := if os() == "windows" { "git-blamr.exe" } else { "git-blamr" }

default: build

build:
    {{ if os() == "windows" { "$env:CGO_ENABLED='0'; go build -trimpath -ldflags '-s -w' -o " + binary + " ./cmd/blamr" } else { "CGO_ENABLED=0 go build -trimpath -ldflags '-s -w' -o " + binary + " ./cmd/blamr" } }}

build-all: build
    {{ if os() == "windows" { "$env:CGO_ENABLED='0'; go build -trimpath -ldflags '-s -w' -o " + git_binary + " ./cmd/git-blamr" } else { "CGO_ENABLED=0 go build -trimpath -ldflags '-s -w' -o " + git_binary + " ./cmd/git-blamr" } }}

fmt:
    go fmt ./...

vet:
    go vet ./...

test:
    go test -v ./...

test-coverage:
    go test -coverprofile=coverage.out ./...

clean:
    {{ if os() == "windows" { "@('blamr.exe', 'git-blamr.exe', 'dist', 'coverage.out') | ForEach-Object { if (Test-Path $_) { Remove-Item -Recurse -Force $_ } }" } else { "rm -rf blamr git-blamr dist coverage.out" } }}
