//! String work as programs do it: joining, splitting, case, searching and replacing.

const WORDS: [&str; 7] = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"];

fn work(n: i64) -> i64 {
    let mut total = 0i64;
    for i in 0..n {
        let parts: Vec<&str> = (0..8).map(|k| WORDS[((i + k) % 7) as usize]).collect();
        let line = parts.join(" ");
        let upper = line.to_uppercase();
        let pieces: Vec<String> = upper.split(' ').map(String::from).collect();
        let gamma = upper.find("GAMMA").map_or(-1, |at| at as i64);
        total += pieces.len() as i64 + gamma + line.matches('a').count() as i64;
        total += line.replace("beta", "b").len() as i64 + line.trim_matches('a').len() as i64;
        if line.starts_with("alpha") {
            total += 1;
        }
    }
    total
}

fn main() {
    let n: i64 = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(2000);
    println!("{}", work(n));
}
