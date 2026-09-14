package main

import "github.com/ajmalbuv/blamr/internal/app"

// version can be customized at compile time via -ldflags "-X main.version=..."
var version = "0.1.0"

func main() {
	app.Run(version)
}
