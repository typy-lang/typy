//! The integer functions of `math`: `factorial`, `isqrt`, `comb`, `perm`, `gcd` and `lcm`, on
//! `int` words, with CPython's results, limits and messages.
#![allow(
    clippy::float_cmp,
    reason = "CPython's algorithms compare doubles exactly, as IEEE 754 defines"
)]

use core::cmp::Ordering;

use super::integer;
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::int::value::Integer;

/// The largest count CPython's integer functions loop over: a C `long`.
const LIMIT: i128 = i64::MAX as i128;

/// Returns an error result of an `int` function.
fn error(kind: ExceptionKind, message: &str) -> Raising<i64> {
    Raising::error(0, raise(kind, message))
}

fn small(value: i128) -> Integer {
    Integer::from_i128(value)
}

/// Returns the product of the integers from `low` to `high`, both included, split in halves
/// so the factors stay of like size.
fn product(low: i128, high: i128) -> Integer {
    if low > high {
        return small(1);
    }
    if high - low < 8 {
        return (low..=high).fold(small(1), |total, factor| total.multiply(&small(factor)));
    }
    let middle = low + (high - low) / 2;
    product(low, middle).multiply(&product(middle + 1, high))
}

/// Returns `n!` for a non-negative `n` of at most a C `long`.
fn factorial_of(n: &Integer) -> Raising<i64> {
    if n.negative {
        return error(
            ExceptionKind::ValueError,
            "factorial() not defined for negative values",
        );
    }
    match n.to_i128().filter(|value| *value <= LIMIT) {
        Some(value) => Raising::ok(product(1, value).into_word()),
        None => error(
            ExceptionKind::OverflowError,
            "factorial() argument should not exceed 9223372036854775807",
        ),
    }
}

/// `math.factorial(n)`.
///
/// # Safety
///
/// `n` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_factorial(n: i64) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    factorial_of(&unsafe { integer(n) })
}

/// `math.isqrt(n)`: the largest integer whose square is at most `n`, by Newton's method from
/// a power of two above the root.
///
/// # Safety
///
/// `n` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_isqrt(n: i64) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    let n = unsafe { integer(n) };
    if n.negative {
        return error(
            ExceptionKind::ValueError,
            "isqrt() argument must be nonnegative",
        );
    }
    if n.is_zero() {
        return Raising::ok(small(0).into_word());
    }
    let bits = crate::int::natural::bit_length(n.magnitude.as_slice());
    let start = Integer {
        negative: false,
        magnitude: crate::int::natural::shift_left(&[1], bits.div_ceil(2)),
    };
    let two = small(2);
    let mut root = start;
    loop {
        let (quotient, _) = n.divide_floor(&root);
        let (next, _) = root.add(&quotient).divide_floor(&two);
        if next.compare(&root) != Ordering::Less {
            return Raising::ok(root.into_word());
        }
        root = next;
    }
}

/// Returns why `n` or `k` of `comb` or `perm` is refused, if it is.
fn check_arguments(n: &Integer, k: &Integer) -> Option<Raising<i64>> {
    if n.negative {
        return Some(error(
            ExceptionKind::ValueError,
            "n must be a non-negative integer",
        ));
    }
    if k.negative {
        return Some(error(
            ExceptionKind::ValueError,
            "k must be a non-negative integer",
        ));
    }
    None
}

/// `math.comb(n, k)`: the number of ways to choose `k` of `n`, 0 when `k > n`.
///
/// # Safety
///
/// Both are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_comb(n: i64, k: i64) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    let (n, k) = unsafe { (integer(n), integer(k)) };
    if let Some(refused) = check_arguments(&n, &k) {
        return refused;
    }
    if k.compare(&n) == Ordering::Greater {
        return Raising::ok(small(0).into_word());
    }
    let rest = n.subtract(&k);
    let smaller = if rest.compare(&k) == Ordering::Less {
        rest
    } else {
        k
    };
    let Some(count) = smaller.to_i128().filter(|count| *count <= LIMIT) else {
        return error(
            ExceptionKind::OverflowError,
            "min(n - k, k) must not exceed 9223372036854775807",
        );
    };
    // Each partial result is itself a binomial coefficient, so each division is exact.
    let start = n.subtract(&small(count));
    let mut result = small(1);
    for step in 1..=count {
        let factor = start.add(&small(step));
        result = result.multiply(&factor).divide_floor(&small(step)).0;
    }
    Raising::ok(result.into_word())
}

/// `math.perm(n, k)`: the number of ordered choices of `k` of `n`, 0 when `k > n`.
///
/// # Safety
///
/// Both are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_perm(n: i64, k: i64) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    let (n, k) = unsafe { (integer(n), integer(k)) };
    if let Some(refused) = check_arguments(&n, &k) {
        return refused;
    }
    if k.compare(&n) == Ordering::Greater {
        return Raising::ok(small(0).into_word());
    }
    let Some(count) = k.to_i128().filter(|count| *count <= LIMIT) else {
        return error(
            ExceptionKind::OverflowError,
            "k must not exceed 9223372036854775807",
        );
    };
    let start = n.subtract(&small(count));
    let mut result = small(1);
    for step in 1..=count {
        result = result.multiply(&start.add(&small(step)));
    }
    Raising::ok(result.into_word())
}

/// Returns the greatest common divisor of two magnitudes, by Euclid's algorithm.
fn greatest_divisor(a: &Integer, b: &Integer) -> Integer {
    let mut a = Integer {
        negative: false,
        magnitude: crate::int::limbs::Limbs::from_slice(a.magnitude.as_slice()),
    };
    let mut b = Integer {
        negative: false,
        magnitude: crate::int::limbs::Limbs::from_slice(b.magnitude.as_slice()),
    };
    while !b.is_zero() {
        let (_, remainder) = a.divide_floor(&b);
        a = b;
        b = remainder;
    }
    a
}

/// `math.gcd(a, b)`, which is never negative.
///
/// # Safety
///
/// Both are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_gcd(a: i64, b: i64) -> i64 {
    // SAFETY: the caller's guarantee.
    let (a, b) = unsafe { (integer(a), integer(b)) };
    greatest_divisor(&a, &b).into_word()
}

/// `math.lcm(a, b)`, which is never negative, and 0 when either is.
///
/// # Safety
///
/// Both are `int` words whose large integers are live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_lcm(a: i64, b: i64) -> i64 {
    // SAFETY: the caller's guarantee.
    let (a, b) = unsafe { (integer(a), integer(b)) };
    if a.is_zero() || b.is_zero() {
        return small(0).into_word();
    }
    let divisor = greatest_divisor(&a, &b);
    let (quotient, _) = a.divide_floor(&divisor);
    let product = quotient.multiply(&b);
    Integer {
        negative: false,
        magnitude: product.magnitude,
    }
    .into_word()
}

/// `math.perm(n)`: `n!`, with `factorial`'s errors.
///
/// # Safety
///
/// `n` is an `int` word whose large integer is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_perm_all(n: i64) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    unsafe { typy_rt_math_factorial(n) }
}
