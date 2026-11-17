//! The k-nucleotide of the Benchmarks Game, over a sequence of N drawn as fasta draws it.

use std::collections::HashMap;

const IM: i64 = 139968;
const IA: i64 = 3877;
const IC: i64 = 29573;

fn sequence(n: usize) -> Vec<u8> {
    let homo_sapiens = [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008];
    let mut table = [0.0; 4];
    let mut total = 0.0;
    for (slot, p) in table.iter_mut().zip(homo_sapiens) {
        total += p;
        *slot = total;
    }
    table[3] = 1.0;
    let mut seed = 42i64;
    (0..n)
        .map(|_| {
            seed = (seed * IA + IC) % IM;
            let drawn = seed as f64 / IM as f64;
            b"ACGT"[table.iter().position(|&limit| drawn < limit).unwrap()]
        })
        .collect()
}

fn counts(text: &[u8], k: usize) -> HashMap<&[u8], i64> {
    let mut found = HashMap::new();
    for fragment in text.windows(k) {
        *found.entry(fragment).or_insert(0) += 1;
    }
    found
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(10000);
    let text = sequence(n);
    for k in 1..=2 {
        let mut ranked: Vec<(i64, &[u8])> =
            counts(&text, k).into_iter().map(|(fragment, count)| (-count, fragment)).collect();
        ranked.sort();
        let total = (text.len() - k + 1) as f64;
        for (negative, fragment) in ranked {
            println!("{} {:.3}", String::from_utf8_lossy(fragment), 100.0 * -negative as f64 / total);
        }
        println!();
    }
    for fragment in ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"] {
        let found = counts(&text, fragment.len()).get(fragment.as_bytes()).copied().unwrap_or(0);
        println!("{found}\t{fragment}");
    }
}
