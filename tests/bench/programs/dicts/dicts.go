// Dictionary work as programs do it: counting words, then ranking the most frequent.
package main

import (
	"fmt"
	"os"
	"sort"
	"strconv"
)

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	found := make(map[string]int)
	seed := 1
	for i := 0; i < n; i++ {
		seed = (seed*1103515245 + 12345) % 2147483648
		found["w"+strconv.Itoa(seed%5000)]++
	}
	type pair struct {
		count int
		word  string
	}
	ranked := make([]pair, 0, len(found))
	for word, count := range found {
		ranked = append(ranked, pair{count, word})
	}
	sort.Slice(ranked, func(i, j int) bool {
		if ranked[i].count != ranked[j].count {
			return ranked[i].count > ranked[j].count
		}
		return ranked[i].word < ranked[j].word
	})
	fmt.Println(len(found))
	for _, p := range ranked[:min(5, len(ranked))] {
		fmt.Printf("%s %d\n", p.word, p.count)
	}
}
