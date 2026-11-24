//! The hashes of values, as CPython computes them for `int`, `float`, `bool`, `None` and
//! tuples, so sets order their items as CPython does (ADR-0010). `str` and `bytes` hash with
//! SipHash-1-3, CPython's function, under a fixed key; CPython draws its key at random, so any
//! key gives an order CPython could give.

use core::sync::atomic::Ordering;

use crate::header::IMMORTAL;
use crate::int::value::Integer;
use crate::string::{NOT_HASHED, StrObject, length};
use crate::word::{inline_value, is_inline};

/// The prime `2**61 - 1` hashes of numbers are reduced by, CPython's `_PyHASH_MODULUS`.
const MODULUS: u64 = (1 << 61) - 1;
/// The number of bits of the modulus, `_PyHASH_BITS`.
const BITS: u32 = 61;
/// The hash of an infinity, `_PyHASH_INF`.
const INFINITY: i64 = 314_159;
/// `hash(None)` in CPython 3.12 and later.
pub(crate) const NONE: i64 = 4_238_894_112;

/// Returns the hash of `item`, of the item kind whose code is the `int` word `kind` and, for a
/// tuple, the descriptor at the address `tuple`, as an `int` word the caller owns: what a set
/// or a dict hashes it by, which `hash()` and the `__hash__` of a dataclass return (LIB-34,
/// CLS-16).
///
/// # Safety
///
/// `item` is a live item of that kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_hash_value(kind: i64, tuple: u64, item: u64) -> i64 {
    let kind = u32::try_from(inline_value(kind)).unwrap_or(u32::MAX);
    let tuple = core::ptr::with_exposed_provenance(usize::try_from(tuple).unwrap_or(0));
    // SAFETY: the caller's guarantee.
    let hash = unsafe { crate::list::ItemType::of(kind, tuple).hash(item) };
    // The hash of an instance is what its `__hash__` gave, which may be outside the inline
    // range.
    crate::word::inline(hash).unwrap_or_else(|| Integer::from_i128(i128::from(hash)).into_word())
}

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

/// Returns the hash of the `int` a class's `__hash__` returns, for the `hash` slot of the
/// class's type object (CLS-07, ADR-0026): the value itself when it fits a word, as CPython's
/// slot keeps it, and the hash of the `int` otherwise; -1 becomes -2. Drops the caller's
/// reference to the `int`.
///
/// # Safety
///
/// `word` is a live `int` word the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_int_hash(word: i64) -> i64 {
    if is_inline(word) {
        return not_minus_one(inline_value(word));
    }
    // SAFETY: a word that is not inline points to a live large integer.
    let fits = unsafe { Integer::from_word(word) }
        .to_i128()
        .and_then(|value| i64::try_from(value).ok());
    // SAFETY: the caller's guarantee.
    let hash = fits.map_or_else(|| unsafe { int(word) }, not_minus_one);
    // SAFETY: the caller owns the reference, which is dropped once.
    unsafe { crate::refcount::typy_rt_int_decrement(word) };
    hash
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
    // A double's 53-bit mantissa is taken in two steps of 28 bits, so the hash stays below
    // 2**56 and never reaches the modulus CPython reduces it by for wider formats.
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
/// The first hash of a string is kept in it, so a key looked up and then stored is hashed
/// once (ADR-0101). An immortal string lives in read-only memory and is hashed each time.
///
/// # Safety
///
/// `text` points to a live string.
pub(crate) unsafe fn text(text: *const StrObject) -> i64 {
    // SAFETY: the caller passes a live string; every index stays inside it.
    unsafe {
        let kept = (*text).hash.load(Ordering::Relaxed);
        if kept != NOT_HASHED {
            return kept;
        }
        let count = length(text);
        if count == 0 {
            return 0;
        }
        let hash = not_minus_one(siphash13(crate::string::narrow::stored(text)).cast_signed());
        // Two threads that hash one string store the same value, so a relaxed store does.
        if (*text).header.reference_count != IMMORTAL {
            (*text).hash.store(hash, Ordering::Relaxed);
        }
        hash
    }
}

/// Returns SipHash-1-3 of `bytes` under a key of zeros: one compression round per word and
/// three finalisation rounds, as CPython's `siphash13`.
fn siphash13(bytes: &[u8]) -> u64 {
    let mut state = SipHash {
        v: [
            0x736f_6d65_7073_6575,
            0x646f_7261_6e64_6f6d,
            0x6c79_6765_6e65_7261,
            0x7465_6462_7974_6573,
        ],
    };
    let (words, rest) = bytes.as_chunks::<8>();
    for word in words {
        state.compress(u64::from_le_bytes(*word));
    }
    // The bytes after the last whole word, then the length's low byte at the top.
    let tail = rest
        .iter()
        .rev()
        .fold(0_u64, |word, byte| (word << 8) | u64::from(*byte));
    state.compress(tail | ((bytes.len() as u64) << 56));
    state.v[2] ^= 0xff;
    for _ in 0..3 {
        state.round();
    }
    state.v[0] ^ state.v[1] ^ state.v[2] ^ state.v[3]
}

/// The state of `SipHash`: four words.
struct SipHash {
    v: [u64; 4],
}

impl SipHash {
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
}

#[cfg(test)]
mod tests;
