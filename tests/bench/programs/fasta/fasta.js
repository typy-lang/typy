// The fasta of the Benchmarks Game: DNA sequences, repeated and drawn from a generator.
"use strict";

const IM = 139968, IA = 3877, IC = 29573, WIDTH = 60;
const ALU =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAG" +
  "ACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCC" +
  "AGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

let seed = 42;
function draw(maximum) {
  seed = (seed * IA + IC) % IM;
  return (maximum * seed) / IM;
}

// Lines are gathered and written in large chunks, as a buffered stream writes them.
let pending = [];
function emit(line) {
  pending.push(line);
  if (pending.length === 4096) flush();
}
function flush() {
  if (pending.length) require("fs").writeSync(1, pending.join("\n") + "\n");
  pending = [];
}

function repeatFasta(header, source, n) {
  emit(header);
  const extended = source + source.slice(0, WIDTH);
  let position = 0;
  while (n > 0) {
    const line = Math.min(WIDTH, n);
    emit(extended.slice(position, position + line));
    position = (position + line) % source.length;
    n -= line;
  }
}

function randomFasta(header, symbols, probabilities, n) {
  emit(header);
  const table = [];
  let total = 0.0;
  for (const p of probabilities) table.push((total += p));
  table[table.length - 1] = 1.0;
  while (n > 0) {
    const size = Math.min(WIDTH, n);
    let line = "";
    for (let i = 0; i < size; i++) {
      const drawn = draw(1.0);
      let k = 0;
      while (drawn >= table[k]) k++;
      line += symbols[k];
    }
    emit(line);
    n -= size;
  }
}

const n = Number(process.argv[2]);
repeatFasta(">ONE Homo sapiens alu", ALU, 2 * n);
randomFasta(">TWO IUB ambiguity codes", "acgtBDHKMNRSVWY",
  [0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02], 3 * n);
randomFasta(">THREE Homo sapiens frequency", "acgt",
  [0.302954942668, 0.1979883004921, 0.1975473066391, 0.3015094502008], 5 * n);
flush();
