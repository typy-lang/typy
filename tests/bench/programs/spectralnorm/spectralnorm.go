// The spectral norm of the Benchmarks Game: the largest eigenvalue of an infinite matrix.
package main

import (
	"fmt"
	"math"
	"os"
	"strconv"
)

func element(i, j int) float64 { return 1.0 / float64((i+j)*(i+j+1)/2+i+1) }

func times(u, v []float64) {
	for i := range v {
		total := 0.0
		for j := range u {
			total += element(i, j) * u[j]
		}
		v[i] = total
	}
}

func timesTransposed(u, v []float64) {
	for i := range v {
		total := 0.0
		for j := range u {
			total += element(j, i) * u[j]
		}
		v[i] = total
	}
}

func timesBoth(u, v, scratch []float64) {
	times(u, scratch)
	timesTransposed(scratch, v)
}

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	u, v, scratch := make([]float64, n), make([]float64, n), make([]float64, n)
	for i := range u {
		u[i] = 1.0
	}
	for k := 0; k < 10; k++ {
		timesBoth(u, v, scratch)
		timesBoth(v, u, scratch)
	}
	var vbv, vv float64
	for i := range u {
		vbv += u[i] * v[i]
		vv += v[i] * v[i]
	}
	fmt.Printf("%.9f\n", math.Sqrt(vbv/vv))
}
