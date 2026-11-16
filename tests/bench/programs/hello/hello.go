// Startup: a program that prints one line.
package main

import (
	"fmt"
	"os"
)

func main() {
	fmt.Printf("hello %d\n", len(os.Args))
}
