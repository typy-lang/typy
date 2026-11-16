// Recursive calls: the Fibonacci numbers by their definition.
package main

import (
	"fmt"
	"os"
	"strconv"
)

func fibonacci(n int) int {
	if n < 2 {
		return n
	}
	return fibonacci(n-1) + fibonacci(n-2)
}

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	fmt.Println(fibonacci(n))
}
