// The k-nucleotide of the Benchmarks Game, over a sequence of N drawn as fasta draws it.
package main

import (
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"
)

const (
	im = 139968
	ia = 3877
	ic = 29573
)

func sequence(n int) string {
	homoSapiens := []float64{0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008}
	table := make([]float64, 4)
	total := 0.0
	for i, p := range homoSapiens {
		total += p
		table[i] = total
	}
	table[3] = 1.0
	var text strings.Builder
	seed := 42
	for i := 0; i < n; i++ {
		seed = (seed*ia + ic) % im
		drawn := float64(seed) / im
		k := 0
		for drawn >= table[k] {
			k++
		}
		text.WriteByte("ACGT"[k])
	}
	return text.String()
}

func counts(text string, k int) map[string]int {
	found := make(map[string]int)
	for i := 0; i+k <= len(text); i++ {
		found[text[i:i+k]]++
	}
	return found
}

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	text := sequence(n)
	for k := 1; k <= 2; k++ {
		type pair struct {
			count    int
			fragment string
		}
		var ranked []pair
		for fragment, count := range counts(text, k) {
			ranked = append(ranked, pair{count, fragment})
		}
		sort.Slice(ranked, func(i, j int) bool {
			if ranked[i].count != ranked[j].count {
				return ranked[i].count > ranked[j].count
			}
			return ranked[i].fragment < ranked[j].fragment
		})
		total := float64(len(text) - k + 1)
		for _, p := range ranked {
			fmt.Printf("%s %.3f\n", p.fragment, 100.0*float64(p.count)/total)
		}
		fmt.Println()
	}
	for _, fragment := range []string{"GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"} {
		fmt.Printf("%d\t%s\n", counts(text, len(fragment))[fragment], fragment)
	}
}
