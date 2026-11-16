// The Mandelbrot set of the Benchmarks Game, as the count of points of an N by N grid in it.
package main

import (
	"fmt"
	"os"
	"strconv"
)

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	found := 0
	for y := 0; y < n; y++ {
		ci := 2.0*float64(y)/float64(n) - 1.0
		for x := 0; x < n; x++ {
			cr := 2.0*float64(x)/float64(n) - 1.5
			zr, zi := 0.0, 0.0
			escaped := false
			for i := 0; i < 50; i++ {
				nextR := zr*zr - zi*zi + cr
				zi = 2.0*zr*zi + ci
				zr = nextR
				if zr*zr+zi*zi > 4.0 {
					escaped = true
					break
				}
			}
			if !escaped {
				found++
			}
		}
	}
	fmt.Println(found)
}
