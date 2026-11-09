//! The hashes of values, as CPython computes them for `int`, `float`, `bool`, `None` and
//! tuples, so sets order their items as CPython does (ADR-0010). `str` and `bytes` hash with
//! SipHash-1-3, CPython's function, under a fixed key; CPython draws its key at random, so any
//! key gives an order CPython could give.

use crate::int::value::Integer;
use crate::string::{StrObject, code_point, length};
use crate::word::{inline_value, is_inline};

/// The prime `2**61 - 1` hashes of numbers are reduced by, CPython's `_PyHASH_MODULUS`.
const MODULUS: u64 = (1 << 61) - 1;
/// The number of bits of the modulus, `_PyHASH_BITS`.
const BITS: u32 = 61;
/// The hash of an infinity, `_PyHASH_INF`.
const INFINITY: i64 = 314_159;
/// `hash(None)` in CPython 3.12 and later.
pub(crate) const NONE: i64 = 4_238_894_112;

/// Returns a hash that is never -1, which CPython keeps to mean an error.
const fn not_minus_one(hash: i64) -> i64 {
    if hash == -1 { -2 } else { hash }
}

/// Returns the hash of an `int` word: its value modulo `2**61 - 1`, with its sign.
///
/// # Safety
///
/// A word that is not inline points to a live large integer.
pub(crate) unsafe fn int(word: i64) -> i64 {
    if is_inline(word) {
        let value = inline_value(word);
        let reduced = i64::try_from(value.unsigned_abs() % MODULUS).unwrap_or(0);
        return not_minus_one(if value < 0 { -reduced } else { reduced });
    }
    // SAFETY: the caller's guarantee.
    let integer = unsafe { Integer::from_word(word) };
    // `2**64` is 8 modulo `2**61 - 1`, so each limb, from the most significant, multiplies the
    // remainder by 8.
    let reduced = integer
        .magnitude
        .as_slice()
        .iter()
        .rev()
        .fold(0_u128, |remainder, limb| {
            (remainder * 8 + u128::from(*limb)) % u128::from(MODULUS)
        });
    let reduced = i64::try_from(reduced).unwrap_or(0);
    not_minus_one(if integer.negative { -reduced } else { reduced })
}

/// Returns `(mantissa, exponent)` with the mantissa in `[0.5, 1)`, as C's `frexp` does for a
/// finite value that is not zero.
fn frexp(value: f64) -> (f64, i32) {
    let bits = value.to_bits();
    let biased = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    if biased == 0 {
        // A subnormal: scale it by 2**64 into the normal range first.
        let (mantissa, exponent) = frexp(value * f64::from_bits(0x43f0_0000_0000_0000));
        return (mantissa, exponent - 64);
    }
    let mantissa = f64::from_bits((bits & !(0x7ff << 52)) | (1022 << 52));
    (mantissa, biased - 1022)
}

/// Returns the hash of a `float`, as CPython's `_Py_HashDouble` computes it: equal to the hash
/// of an `int` of the same value. A NaN hashes to 0; CPython hashes it by its identity, which a
/// TyPy float does not have (OI-18).
#[allow(
    clippy::float_cmp,
    reason = "the loop runs until the mantissa is exactly used up, as CPython's does"
)]
pub(crate) fn float(value: f64) -> i64 {
    if value.is_nan() {
        return 0;
    }
    if value.is_infinite() {
        return if value > 0.0 { INFINITY } else { -INFINITY };
    }
    if value == 0.0 {
        return 0;
    }
    let (mut mantissa, mut exponent) = frexp(value.abs());
    let mut hash: u64 = 0;
    while mantissa != 0.0 {
        hash = ((hash << 28) & MODULUS) | (hash >> (BITS - 28));
        mantissa *= 268_435_456.0;
        exponent -= 28;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the mantissa is below 2**28 and not negative, so this takes its integer part"
        )]
        let integer = mantissa as u64;
        #[allow(
            clippy::cast_precision_loss,
            reason = "the integer part is below 2**28, which a double holds exactly"
        )]
        let whole = integer as f64;
        mantissa -= whole;
        hash += integer;
        if hash >= MODULUS {
            hash -= MODULUS;
        }
    }
    let bits = i32::try_from(BITS).unwrap_or(61);
    let shift = if exponent >= 0 {
        exponent % bits
    } else {
        bits - 1 - ((-1 - exponent) % bits)
    };
    let shift = u32::try_from(shift).unwrap_or(0);
    hash = ((hash << shift) & MODULUS) | (hash >> (BITS - shift));
    let hash = i64::try_from(hash).unwrap_or(0);
    not_minus_one(if value < 0.0 { -hash } else { hash })
}

/// Returns the hash of a tuple from the hashes of its items, CPython's xxHash-based `tuplehash`.
pub(crate) fn tuple(items: impl Iterator<Item = i64>, length: usize) -> i64 {
    const PRIME_1: u64 = 11_400_714_785_074_694_791;
    const PRIME_2: u64 = 14_029_467_366_897_019_727;
    const PRIME_5: u64 = 2_870_177_450_012_600_261;
    let mut accumulator = PRIME_5;
    for lane in items {
        accumulator = accumulator.wrapping_add(lane.cast_unsigned().wrapping_mul(PRIME_2));
        accumulator = accumulator.rotate_left(31);
        accumulator = accumulator.wrapping_mul(PRIME_1);
    }
    let length = u64::try_from(length).unwrap_or(u64::MAX);
    accumulator = accumulator.wrapping_add(length ^ (PRIME_5 ^ 0x0035_d373));
    if accumulator == u64::MAX {
        return 1_546_275_796;
    }
    accumulator.cast_signed()
}

/// Returns the hash of a `str` or `bytes`: SipHash-1-3 under a key of zeros of the code points
/// as CPython stores them, one, two or four bytes each by the widest, so the hash is the one
/// CPython gives with `PYTHONHASHSEED=0`. An empty text hashes to 0, as in CPython.
///
/// # Safety
///
/// `text` points to a live string.
pub(crate) unsafe fn text(text: *const StrObject) -> i64 {
    // SAFETY: the caller passes a live string; every index stays inside it.
    unsafe {
        let count = length(text);
        if count == 0 {
            return 0;
        }
        let width = (*text).kind as usize;
        let mut state = SipHash::new();
        for index in 0..count {
            state.write(&code_point(text, index).to_le_bytes()[..width]);
        }
        not_minus_one(state.finish().cast_signed())
    }
}

/// `SipHash` with one compression round per word and three finalisation rounds.
struct SipHash {
    v: [u64; 4],
    tail: u64,
    filled: u32,
    length: u64,
}

impl SipHash {
    const fn new() -> Self {
        Self {
            v: [
                0x736f_6d65_7073_6575,
                0x646f_7261_6e64_6f6d,
                0x6c79_6765_6e65_7261,
                0x7465_6462_7974_6573,
            ],
            tail: 0,
            filled: 0,
            length: 0,
        }
    }

    fn round(&mut self) {
        let v = &mut self.v;
        v[0] = v[0].wrapping_add(v[1]);
        v[1] = v[1].rotate_left(13) ^ v[0];
        v[0] = v[0].rotate_left(32);
        v[2] = v[2].wrapping_add(v[3]);
        v[3] = v[3].rotate_left(16) ^ v[2];
        v[0] = v[0].wrapping_add(v[3]);
        v[3] = v[3].rotate_left(21) ^ v[0];
        v[2] = v[2].wrapping_add(v[1]);
        v[1] = v[1].rotate_left(17) ^ v[2];
        v[2] = v[2].rotate_left(32);
    }

    fn compress(&mut self, word: u64) {
        self.v[3] ^= word;
        self.round();
        self.v[0] ^= word;
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.tail |= u64::from(byte) << (8 * self.filled);
            self.filled += 1;
            self.length += 1;
            if self.filled == 8 {
                let word = self.tail;
                self.compress(word);
                self.tail = 0;
                self.filled = 0;
            }
        }
    }

    fn finish(mut self) -> u64 {
        let last = self.tail | (self.length << 56);
        self.compress(last);
        self.v[2] ^= 0xff;
        for _ in 0..3 {
            self.round();
        }
        self.v[0] ^ self.v[1] ^ self.v[2] ^ self.v[3]
    }
}

#[cfg(test)]
mod tests;
