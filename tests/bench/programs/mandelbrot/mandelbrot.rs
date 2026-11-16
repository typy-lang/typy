//! The Mandelbrot set of the Benchmarks Game, as the count of points of an N by N grid in it.

fn main() {
    let n: i64 = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(200);
    let mut found = 0i64;
    for y in 0..n {
        let ci = 2.0 * y as f64 / n as f64 - 1.0;
        for x in 0..n {
            let cr = 2.0 * x as f64 / n as f64 - 1.5;
            let (mut zr, mut zi) = (0.0f64, 0.0f64);
            let mut escaped = false;
            for _ in 0..50 {
                let next_r = zr * zr - zi * zi + cr;
                zi = 2.0 * zr * zi + ci;
                zr = next_r;
                if zr * zr + zi * zi > 4.0 {
                    escaped = true;
                    break;
                }
            }
            if !escaped {
                found += 1;
            }
        }
    }
    println!("{found}");
}
