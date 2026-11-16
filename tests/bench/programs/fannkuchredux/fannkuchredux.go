// The fannkuch-redux of the Benchmarks Game: flips of every permutation of N items.
package main

import (
	"fmt"
	"os"
	"strconv"
)

func fannkuch(n int) (int, int) {
	first, perm, count := make([]int, n), make([]int, n), make([]int, n)
	for i := range first {
		first[i] = i
	}
	maxFlips, checksum, permutations, r := 0, 0, 0, n
	for {
		for ; r != 1; r-- {
			count[r-1] = r
		}
		copy(perm, first)
		flips := 0
		for k := perm[0]; k != 0; k = perm[0] {
			for i, j := 0, k; i < j; i, j = i+1, j-1 {
				perm[i], perm[j] = perm[j], perm[i]
			}
			flips++
		}
		if flips > maxFlips {
			maxFlips = flips
		}
		if permutations%2 == 0 {
			checksum += flips
		} else {
			checksum -= flips
		}
		for {
			if r == n {
				return checksum, maxFlips
			}
			head := first[0]
			copy(first[:r], first[1:r+1])
			first[r] = head
			count[r]--
			if count[r] > 0 {
				break
			}
			r++
		}
		permutations++
	}
}

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	checksum, maxFlips := fannkuch(n)
	fmt.Printf("%d\nPfannkuchen(%d) = %d\n", checksum, n, maxFlips)
}
