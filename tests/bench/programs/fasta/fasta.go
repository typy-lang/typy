// The fasta of the Benchmarks Game: DNA sequences, repeated and drawn from a generator.
package main

import (
	"bufio"
	"os"
	"strconv"
)

const (
	im    = 139968
	ia    = 3877
	ic    = 29573
	width = 60
	alu   = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAG" +
		"ACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCC" +
		"AGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
		"AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
)

type random struct{ seed int }

func (r *random) draw(maximum float64) float64 {
	r.seed = (r.seed*ia + ic) % im
	return maximum * float64(r.seed) / im
}

func repeatFasta(out *bufio.Writer, header, source string, n int) {
	out.WriteString(header + "\n")
	extended := source + source[:width]
	position := 0
	for n > 0 {
		line := min(width, n)
		out.WriteString(extended[position:position+line] + "\n")
		position = (position + line) % len(source)
		n -= line
	}
}

func randomFasta(out *bufio.Writer, header, symbols string, probabilities []float64, n int, r *random) {
	out.WriteString(header + "\n")
	table := make([]float64, len(probabilities))
	total := 0.0
	for i, p := range probabilities {
		total += p
		table[i] = total
	}
	table[len(table)-1] = 1.0
	line := make([]byte, 0, width+1)
	for n > 0 {
		size := min(width, n)
		line = line[:0]
		for i := 0; i < size; i++ {
			drawn := r.draw(1.0)
			k := 0
			for drawn >= table[k] {
				k++
			}
			line = append(line, symbols[k])
		}
		line = append(line, '\n')
		out.Write(line)
		n -= size
	}
}

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	out := bufio.NewWriter(os.Stdout)
	defer out.Flush()
	repeatFasta(out, ">ONE Homo sapiens alu", alu, 2*n)
	r := &random{42}
	randomFasta(out, ">TWO IUB ambiguity codes", "acgtBDHKMNRSVWY",
		[]float64{0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02}, 3*n, r)
	randomFasta(out, ">THREE Homo sapiens frequency", "acgt",
		[]float64{0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008}, 5*n, r)
}
