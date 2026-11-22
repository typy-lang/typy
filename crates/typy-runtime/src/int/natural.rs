//! Arithmetic on magnitudes: sequences of 64-bit limbs, least significant first, without leading zeros.

use core::cmp::Ordering;

use super::limbs::Limbs;

/// Compares two magnitudes.
pub(crate) fn compare(left: &[u64], right: &[u64]) -> Ordering {
    left.len()
        .cmp(&right.len())
        .then_with(|| left.iter().rev().cmp(right.iter().rev()))
}

/// Returns `left + right`.
pub(crate) fn add(left: &[u64], right: &[u64]) -> Limbs {
    let (long, short) = if left.len() >= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    let mut result = Limbs::new();
    let mut carry = 0_u128;
    for (index, limb) in long.iter().enumerate() {
        let sum = u128::from(*limb) + u128::from(short.get(index).copied().unwrap_or(0)) + carry;
        result.push(low(sum));
        carry = sum >> 64;
    }
    if carry > 0 {
        result.push(low(carry));
    }
    result
}

/// Returns `left - right`, where `left` is at least `right`.
pub(crate) fn subtract(left: &[u64], right: &[u64]) -> Limbs {
    let mut result = Limbs::new();
    let mut borrow = 0_u64;
    for (index, limb) in left.iter().enumerate() {
        let (difference, first) = limb.overflowing_sub(right.get(index).copied().unwrap_or(0));
        let (difference, second) = difference.overflowing_sub(borrow);
        result.push(difference);
        borrow = u64::from(first || second);
    }
    result.trim();
    result
}

/// Returns `left * right`.
pub(crate) fn multiply(left: &[u64], right: &[u64]) -> Limbs {
    let mut result = Limbs::zeros(left.len() + right.len());
    let out = result.as_mut_slice();
    for (i, a) in left.iter().enumerate() {
        let mut carry = 0_u128;
        for (j, b) in right.iter().enumerate() {
            let product = u128::from(*a) * u128::from(*b) + u128::from(out[i + j]) + carry;
            out[i + j] = low(product);
            carry = product >> 64;
        }
        out[i + right.len()] = low(carry);
    }
    result.trim();
    result
}

/// Returns `value * factor + addend`.
pub(crate) fn multiply_add_small(value: &[u64], factor: u64, addend: u64) -> Limbs {
    let mut result = Limbs::new();
    let mut carry = u128::from(addend);
    for limb in value {
        let product = u128::from(*limb) * u128::from(factor) + carry;
        result.push(low(product));
        carry = product >> 64;
    }
    if carry > 0 {
        result.push(low(carry));
    }
    result.trim();
    result
}

/// A limb with its top bit set, and its reciprocal, `floor((2**128 - 1) / divisor) - 2**64`, by
/// which a two-limb number is divided with two multiplications in place of a 128-bit division,
/// as Moller and Granlund's division by invariant integers does.
#[derive(Clone, Copy)]
pub(crate) struct Reciprocal {
    divisor: u64,
    inverse: u64,
}

impl Reciprocal {
    /// Returns the reciprocal of `divisor`, whose top bit is set.
    pub(crate) fn of(divisor: u64) -> Self {
        Self {
            divisor,
            inverse: low(u128::MAX / u128::from(divisor)),
        }
    }

    /// Returns `(high * 2**64 + low_limb) / divisor` and the remainder; `high` is below the
    /// divisor, so the quotient fits a limb.
    pub(crate) fn divide(self, high: u64, low_limb: u64) -> (u64, u64) {
        let product = u128::from(self.inverse) * u128::from(high);
        let estimate = product.wrapping_add((u128::from(high) << 64) | u128::from(low_limb));
        let mut quotient = low(estimate >> 64).wrapping_add(1);
        let mut remainder = low_limb.wrapping_sub(quotient.wrapping_mul(self.divisor));
        if remainder > low(estimate) {
            quotient = quotient.wrapping_sub(1);
            remainder = remainder.wrapping_add(self.divisor);
        }
        if remainder >= self.divisor {
            quotient += 1;
            remainder -= self.divisor;
        }
        (quotient, remainder)
    }
}

/// Divides by a single limb, returning the quotient and the remainder.
pub(crate) fn divide_small(value: &[u64], divisor: u64) -> (Limbs, u64) {
    // Both are shifted until the divisor's top bit is set, which leaves the quotient as it is
    // and the remainder shifted the same way.
    let shift = divisor.leading_zeros();
    let reciprocal = Reciprocal::of(divisor << shift);
    let shifted = shift_left(value, u64::from(shift));
    let shifted = shifted.as_slice();
    let mut quotient = Limbs::zeros(shifted.len());
    let mut remainder = 0_u64;
    for index in (0..shifted.len()).rev() {
        let (digit, rest) = reciprocal.divide(remainder, shifted[index]);
        quotient.as_mut_slice()[index] = digit;
        remainder = rest;
    }
    quotient.trim();
    (quotient, remainder >> shift)
}

/// Returns the quotient and remainder of magnitudes by Knuth's algorithm D; `divisor` is not zero.
pub(crate) fn divide(dividend: &[u64], divisor: &[u64]) -> (Limbs, Limbs) {
    if compare(dividend, divisor) == Ordering::Less {
        return (Limbs::new(), Limbs::from_slice(dividend));
    }
    if divisor.len() == 1 {
        let (quotient, remainder) = divide_small(dividend, divisor[0]);
        return (quotient, Limbs::from_slice(&[remainder]));
    }
    let shift = divisor[divisor.len() - 1].leading_zeros();
    let v = shift_left(divisor, u64::from(shift));
    let mut u = shift_left(dividend, u64::from(shift));
    let n = v.as_slice().len();
    if u.as_slice().len() == dividend.len() {
        u.push(0);
    }
    let m = u.as_slice().len() - n;
    let mut quotient = Limbs::zeros(m);
    let v = v.as_slice();
    let base = 1_u128 << 64;
    let reciprocal = Reciprocal::of(v[n - 1]);
    for j in (0..m).rev() {
        let u_slice = u.as_mut_slice();
        let (high, low_limb) = (u_slice[j + n], u_slice[j + n - 1]);
        // The top limb of the remainder is never above the divisor's; when it equals it, the
        // quotient's estimate is one less than the base.
        let (mut estimate, mut rest) = if high < v[n - 1] {
            let (digit, rest) = reciprocal.divide(high, low_limb);
            (u128::from(digit), u128::from(rest))
        } else {
            let numerator = (u128::from(high) << 64) | u128::from(low_limb);
            let estimate = base - 1;
            (estimate, numerator - estimate * u128::from(v[n - 1]))
        };
        // The estimate is at most two too large; once the remainder of the estimate reaches the
        // base it is at most one too large, which the subtraction below corrects.
        while rest < base
            && estimate * u128::from(v[n - 2]) > ((rest << 64) | u128::from(u_slice[j + n - 2]))
        {
            estimate -= 1;
            rest += u128::from(v[n - 1]);
        }
        let mut borrow = 0_i128;
        let mut carry = 0_u128;
        for i in 0..n {
            let product = estimate * u128::from(v[i]) + carry;
            carry = product >> 64;
            let difference = i128::from(u_slice[i + j]) - i128::from(low(product)) + borrow;
            u_slice[i + j] = low_signed(difference);
            borrow = difference >> 64;
        }
        let difference = i128::from(u_slice[j + n]) - i128::try_from(carry).unwrap_or(0) + borrow;
        u_slice[j + n] = low_signed(difference);
        if difference < 0 {
            estimate -= 1;
            let mut carry = 0_u128;
            for i in 0..n {
                let sum = u128::from(u_slice[i + j]) + u128::from(v[i]) + carry;
                u_slice[i + j] = low(sum);
                carry = sum >> 64;
            }
            u_slice[j + n] = u_slice[j + n].wrapping_add(low(carry));
        }
        quotient.as_mut_slice()[j] = low(estimate);
    }
    quotient.trim();
    u.trim();
    let remainder = shift_right(u.as_slice(), u64::from(shift));
    (quotient, remainder)
}

/// Returns `value << bits`.
pub(crate) fn shift_left(value: &[u64], bits: u64) -> Limbs {
    let whole = usize::try_from(bits / 64).unwrap_or(usize::MAX);
    let part = bits % 64;
    let mut result = Limbs::zeros(whole);
    let mut carry = 0_u64;
    for limb in value {
        if part == 0 {
            result.push(*limb);
        } else {
            result.push((limb << part) | carry);
            carry = limb >> (64 - part);
        }
    }
    if carry > 0 {
        result.push(carry);
    }
    result.trim();
    result
}

/// Returns `value >> bits`, rounding toward zero.
pub(crate) fn shift_right(value: &[u64], bits: u64) -> Limbs {
    let whole = usize::try_from(bits / 64).unwrap_or(usize::MAX);
    if whole >= value.len() {
        return Limbs::new();
    }
    let part = bits % 64;
    let mut result = Limbs::new();
    for index in whole..value.len() {
        let high = if part == 0 {
            0
        } else {
            value.get(index + 1).map_or(0, |next| next << (64 - part))
        };
        result.push((value[index] >> part) | high);
    }
    result.trim();
    result
}

/// Returns the number of significant bits.
pub(crate) fn bit_length(value: &[u64]) -> u64 {
    value.last().map_or(0, |top| {
        (value.len() as u64 - 1) * 64 + u64::from(64 - top.leading_zeros())
    })
}

/// Returns true when any of the lowest `bits` bits is set.
pub(crate) fn any_low_bits(value: &[u64], bits: u64) -> bool {
    let whole = usize::try_from(bits / 64).unwrap_or(usize::MAX);
    let part = bits % 64;
    value.iter().take(whole).any(|limb| *limb != 0)
        || value
            .get(whole)
            .is_some_and(|limb| part > 0 && limb & ((1 << part) - 1) != 0)
}

fn low(value: u128) -> u64 {
    u64::try_from(value & u128::from(u64::MAX)).unwrap_or(0)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the low 64 bits of a two's complement value"
)]
fn low_signed(value: i128) -> u64 {
    value as u64
}

#[cfg(test)]
mod tests;
