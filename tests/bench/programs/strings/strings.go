// String work as programs do it: joining, splitting, case, searching and replacing.
package main

import (
	"fmt"
	"os"
	"strconv"
	"strings"
)

var words = []string{"alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"}

func work(n int) int {
	total := 0
	for i := 0; i < n; i++ {
		parts := make([]string, 8)
		for k := range parts {
			parts[k] = words[(i+k)%len(words)]
		}
		line := strings.Join(parts, " ")
		upper := strings.ToUpper(line)
		pieces := strings.Split(upper, " ")
		total += len(pieces) + strings.Index(upper, "GAMMA") + strings.Count(line, "a")
		total += len(strings.ReplaceAll(line, "beta", "b")) + len(strings.Trim(line, "a"))
		if strings.HasPrefix(line, "alpha") {
			total++
		}
	}
	return total
}

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	fmt.Println(work(n))
}
