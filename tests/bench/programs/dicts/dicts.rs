//! Dictionary work as programs do it: counting words, then ranking the most frequent.

use std::collections::HashMap;

fn main() {
    let n: i64 = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(20000);
    let mut found: HashMap<String, i64> = HashMap::new();
    let mut seed = 1i64;
    for _ in 0..n {
        seed = (seed * 1103515245 + 12345) % 2147483648;
        *found.entry(format!("w{}", seed % 5000)).or_insert(0) += 1;
    }
    let mut ranked: Vec<(i64, &String)> = found.iter().map(|(word, count)| (-count, word)).collect();
    ranked.sort();
    println!("{}", found.len());
    for (negative, word) in ranked.iter().take(5) {
        println!("{word} {}", -negative);
    }
}
