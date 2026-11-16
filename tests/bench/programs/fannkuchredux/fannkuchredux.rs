//! The fannkuch-redux of the Benchmarks Game: flips of every permutation of N items.

fn fannkuch(n: usize) -> (i64, i64) {
    let mut first: Vec<usize> = (0..n).collect();
    let mut perm = vec![0; n];
    let mut count = vec![0; n];
    let (mut max_flips, mut checksum, mut permutations) = (0i64, 0i64, 0i64);
    let mut r = n;
    loop {
        while r != 1 {
            count[r - 1] = r;
            r -= 1;
        }
        perm.copy_from_slice(&first);
        let mut flips = 0i64;
        let mut k = perm[0];
        while k != 0 {
            perm[..=k].reverse();
            flips += 1;
            k = perm[0];
        }
        max_flips = max_flips.max(flips);
        checksum += if permutations % 2 == 0 { flips } else { -flips };
        loop {
            if r == n {
                return (checksum, max_flips);
            }
            first[..=r].rotate_left(1);
            count[r] -= 1;
            if count[r] > 0 {
                break;
            }
            r += 1;
        }
        permutations += 1;
    }
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(7);
    let (checksum, max_flips) = fannkuch(n);
    println!("{checksum}\nPfannkuchen({n}) = {max_flips}");
}
