//! The spectral norm of the Benchmarks Game: the largest eigenvalue of an infinite matrix.

fn element(i: usize, j: usize) -> f64 {
    1.0 / ((i + j) * (i + j + 1) / 2 + i + 1) as f64
}

fn times(u: &[f64], v: &mut [f64]) {
    for (i, out) in v.iter_mut().enumerate() {
        *out = u.iter().enumerate().map(|(j, x)| element(i, j) * x).sum();
    }
}

fn times_transposed(u: &[f64], v: &mut [f64]) {
    for (i, out) in v.iter_mut().enumerate() {
        *out = u.iter().enumerate().map(|(j, x)| element(j, i) * x).sum();
    }
}

fn times_both(u: &[f64], v: &mut [f64], scratch: &mut [f64]) {
    times(u, scratch);
    times_transposed(scratch, v);
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(100);
    let mut u = vec![1.0; n];
    let mut v = vec![0.0; n];
    let mut scratch = vec![0.0; n];
    for _ in 0..10 {
        times_both(&u, &mut v, &mut scratch);
        times_both(&v, &mut u, &mut scratch);
    }
    let vbv: f64 = u.iter().zip(&v).map(|(a, b)| a * b).sum();
    let vv: f64 = v.iter().map(|b| b * b).sum();
    println!("{:.9}", (vbv / vv).sqrt());
}
