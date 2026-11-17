//! Recursive calls: the Fibonacci numbers by their definition.

fn fibonacci(n: i64) -> i64 {
    if n < 2 { n } else { fibonacci(n - 1) + fibonacci(n - 2) }
}

fn main() {
    let n: i64 = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(22);
    println!("{}", fibonacci(n));
}
