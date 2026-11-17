// The binary trees of the Benchmarks Game: many short-lived trees, allocated and freed.
package main

import (
	"fmt"
	"os"
	"strconv"
)

type node struct{ left, right *node }

func build(depth int) *node {
	if depth == 0 {
		return &node{}
	}
	return &node{build(depth - 1), build(depth - 1)}
}

func check(n *node) int {
	if n.left == nil || n.right == nil {
		return 1
	}
	return 1 + check(n.left) + check(n.right)
}

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	maxDepth := max(6, n)
	stretch := maxDepth + 1
	fmt.Printf("stretch tree of depth %d\t check: %d\n", stretch, check(build(stretch)))
	longLived := build(maxDepth)
	for depth := 4; depth <= maxDepth; depth += 2 {
		iterations := 1 << (maxDepth - depth + 4)
		total := 0
		for i := 0; i < iterations; i++ {
			total += check(build(depth))
		}
		fmt.Printf("%d\t trees of depth %d\t check: %d\n", iterations, depth, total)
	}
	fmt.Printf("long lived tree of depth %d\t check: %d\n", maxDepth, check(longLived))
}
