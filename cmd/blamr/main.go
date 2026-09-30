package main

import "github.com/ajmalbuv/blamr/internal/app"

// version can be customized at compile time via -ldflags "-X main.version=..."
var version = "dev"

func main() {
	app.Run(version)
}
