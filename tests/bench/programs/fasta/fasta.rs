//! The fasta of the Benchmarks Game: DNA sequences, repeated and drawn from a generator.

use std::io::{BufWriter, Write};

const IM: i64 = 139968;
const IA: i64 = 3877;
const IC: i64 = 29573;
const WIDTH: usize = 60;
const ALU: &str = concat!(
    "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAG",
    "ACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCC",
    "AGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC",
    "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
);

struct Random {
    seed: i64,
}

impl Random {
    fn draw(&mut self, maximum: f64) -> f64 {
        self.seed = (self.seed * IA + IC) % IM;
        maximum * self.seed as f64 / IM as f64
    }
}

fn repeat_fasta(out: &mut impl Write, header: &str, source: &str, mut n: usize) {
    writeln!(out, "{header}").unwrap();
    let extended = format!("{source}{}", &source[..WIDTH]);
    let mut position = 0;
    while n > 0 {
        let line = n.min(WIDTH);
        writeln!(out, "{}", &extended[position..position + line]).unwrap();
        position = (position + line) % source.len();
        n -= line;
    }
}

fn random_fasta(
    out: &mut impl Write,
    header: &str,
    symbols: &[u8],
    probabilities: &[f64],
    mut n: usize,
    random: &mut Random,
) {
    writeln!(out, "{header}").unwrap();
    let mut table: Vec<f64> = probabilities
        .iter()
        .scan(0.0, |total, p| {
            *total += p;
            Some(*total)
        })
        .collect();
    *table.last_mut().unwrap() = 1.0;
    let mut line = Vec::with_capacity(WIDTH + 1);
    while n > 0 {
        let size = n.min(WIDTH);
        line.clear();
        for _ in 0..size {
            let drawn = random.draw(1.0);
            let k = table.iter().position(|&limit| drawn < limit).unwrap();
            line.push(symbols[k]);
        }
        line.push(b'\n');
        out.write_all(&line).unwrap();
        n -= size;
    }
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(1000);
    let iub = [0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02];
    let homo_sapiens = [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008];
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    repeat_fasta(&mut out, ">ONE Homo sapiens alu", ALU, 2 * n);
    let mut random = Random { seed: 42 };
    random_fasta(&mut out, ">TWO IUB ambiguity codes", b"acgtBDHKMNRSVWY", &iub, 3 * n, &mut random);
    random_fasta(&mut out, ">THREE Homo sapiens frequency", b"acgt", &homo_sapiens, 5 * n, &mut random);
}
