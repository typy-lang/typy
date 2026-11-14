//! The functions of `math` over iterables of numbers: `fsum`, `dist`, `prod` and `sumprod`,
//! each given lists of `int`, `bool` or `float` items and computed as CPython computes them
//! (LIB-12).
#![allow(
    clippy::float_cmp,
    reason = "CPython's algorithms compare doubles exactly, as IEEE 754 defines"
)]

use super::c;
use super::{integer, ok};
use crate::Raising;
use crate::exception::{ExceptionKind, raise};
use crate::int::value::Integer;
use crate::libm::absolute;
use crate::list::ItemKind;
use crate::list::ListObject;

/// Returns `ValueError` with `message`.
fn value_error<T>(value: T, message: &str) -> Raising<T> {
    Raising::error(value, raise(ExceptionKind::ValueError, message))
}

/// Returns item `index` of a list of numbers as a `float`, as CPython converts it: an `int`
/// too large for one raises `OverflowError`.
///
/// # Safety
///
/// `list` is a live list of `int`, `bool` or `float` items, and `index` is below its length.
unsafe fn real(list: &ListObject, index: usize) -> Raising<f64> {
    // SAFETY: the caller keeps the index below the length.
    let bits = unsafe { list.item(index) };
    match list.item_type().kind() {
        ItemKind::Float => ok(f64::from_bits(bits)),
        ItemKind::Bool => ok(if bits == 0 { 0.0 } else { 1.0 }),
        #[allow(clippy::cast_possible_wrap, reason = "an `int` item is an `int` word")]
        // SAFETY: the list holds the large integer of its item.
        _ => unsafe { crate::float::typy_rt_int_to_float(bits as i64) },
    }
}

/// Returns item `index` of a list of `int` or `bool` items as an integer.
///
/// # Safety
///
/// As `real`, for a list of `int` or `bool` items.
unsafe fn whole(list: &ListObject, index: usize) -> Integer {
    // SAFETY: the caller keeps the index below the length.
    let bits = unsafe { list.item(index) };
    if list.item_type().kind() == ItemKind::Bool {
        return Integer::from_i128(i128::from(bits != 0));
    }
    #[allow(clippy::cast_possible_wrap, reason = "an `int` item is an `int` word")]
    // SAFETY: the list holds the large integer of its item.
    unsafe {
        integer(bits as i64)
    }
}

/// A buffer of `count` doubles from the runtime's allocator, freed when dropped.
struct Doubles {
    start: *mut f64,
    count: usize,
}

impl Doubles {
    fn new(count: usize) -> Self {
        let size = crate::allocator::array_size(count.max(1), size_of::<f64>(), 0);
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "malloc returns memory aligned for every type"
        )]
        let start = crate::allocator::allocate(size).cast::<f64>();
        Self { start, count }
    }

    fn as_slice(&mut self) -> &mut [f64] {
        // SAFETY: the buffer holds `count` doubles, each written before it is read.
        unsafe {
            for index in 0..self.count {
                self.start.add(index).write(0.0);
            }
            core::slice::from_raw_parts_mut(self.start, self.count)
        }
    }
}

impl Drop for Doubles {
    fn drop(&mut self) {
        crate::allocator::free(self.start.cast());
    }
}

/// `math.fsum(values)`: the sum, rounded once, by Shewchuk's partial sums as CPython keeps
/// them.
///
/// # Safety
///
/// `values` is a live list of `int`, `bool` or `float` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_fsum(values: *const ListObject) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    let list = unsafe { &*values };
    // Each item adds at most one partial.
    let mut buffer = Doubles::new(list.length + 1);
    let partials = buffer.as_slice();
    let mut count = 0;
    let (mut special_sum, mut inf_sum) = (0.0_f64, 0.0_f64);
    for index in 0..list.length {
        // SAFETY: the index is below the length.
        let item = unsafe { real(list, index) };
        if !item.exception.is_null() {
            return item;
        }
        let saved = item.value;
        let mut x = item.value;
        let mut kept = 0;
        for position in 0..count {
            let mut y = partials[position];
            if absolute(x) < absolute(y) {
                core::mem::swap(&mut x, &mut y);
            }
            let high = x + y;
            let low = y - (high - x);
            if low != 0.0 {
                partials[kept] = low;
                kept += 1;
            }
            x = high;
        }
        count = kept;
        if x == 0.0 {
            continue;
        }
        if x.is_finite() {
            partials[count] = x;
            count += 1;
            continue;
        }
        // A sum that is not finite comes from an overflow, or from a NaN or an infinity among
        // the values.
        if saved.is_finite() {
            return Raising::error(
                0.0,
                raise(
                    ExceptionKind::OverflowError,
                    "intermediate overflow in fsum",
                ),
            );
        }
        if saved.is_infinite() {
            inf_sum += saved;
        }
        special_sum += saved;
        count = 0;
    }
    if special_sum != 0.0 {
        if inf_sum.is_nan() {
            return value_error(0.0, "-inf + inf in fsum");
        }
        return ok(special_sum);
    }
    ok(exact_sum(&partials[..count]))
}

/// Returns the sum of the partials, from the largest, rounded half to even across them.
fn exact_sum(partials: &[f64]) -> f64 {
    let Some((&last, rest)) = partials.split_last() else {
        return 0.0;
    };
    let mut high = last;
    let mut low = 0.0;
    let mut remaining = rest.len();
    while remaining > 0 {
        remaining -= 1;
        let (x, y) = (high, rest[remaining]);
        high = x + y;
        low = y - (high - x);
        if low != 0.0 {
            break;
        }
    }
    // A low part of the same sign as the next partial moves a tie to the other side.
    if remaining > 0
        && ((low < 0.0 && rest[remaining - 1] < 0.0) || (low > 0.0 && rest[remaining - 1] > 0.0))
    {
        let y = low * 2.0;
        let x = high + y;
        if y == x - high {
            high = x;
        }
    }
    high
}

/// `math.dist(p, q)`: the Euclidean distance between two points of as many coordinates.
///
/// # Safety
///
/// Both are live lists of `int`, `bool` or `float` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_dist(
    p: *const ListObject,
    q: *const ListObject,
) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    let (p, q) = unsafe { (&*p, &*q) };
    if p.length != q.length {
        return value_error(0.0, "both points must have the same number of dimensions");
    }
    let mut buffer = Doubles::new(p.length);
    let differences = buffer.as_slice();
    let (mut max, mut found_nan) = (0.0_f64, false);
    for (index, slot) in differences.iter_mut().enumerate() {
        // SAFETY: the index is below both lengths.
        let (x, y) = unsafe { (real(p, index), real(q, index)) };
        for each in [&x, &y] {
            if !each.exception.is_null() {
                return Raising::error(0.0, each.exception);
            }
        }
        let difference = absolute(x.value - y.value);
        *slot = difference;
        found_nan |= difference.is_nan();
        if difference > max {
            max = difference;
        }
    }
    ok(super::special::vector_norm(differences, max, found_nan))
}

/// `math.prod(values, start=start)` of `int` or `bool` items and an `int` start: the exact
/// product, an `int`.
///
/// # Safety
///
/// `values` is a live list of `int` or `bool` items, and `start` an `int` word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_prod_int(values: *const ListObject, start: i64) -> i64 {
    // SAFETY: the caller's guarantee.
    let (list, mut product) = unsafe { (&*values, integer(start)) };
    for index in 0..list.length {
        // SAFETY: the index is below the length.
        product = product.multiply(&unsafe { whole(list, index) });
    }
    product.into_word()
}

/// `math.prod(values, start=start)` of at least one item with a `float` among the items or
/// the start, given as a `float`: each item multiplies the product as a `float`.
///
/// # Safety
///
/// `values` is a live list of `int`, `bool` or `float` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_prod_float(
    values: *const ListObject,
    start: f64,
) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    let list = unsafe { &*values };
    let mut product = start;
    for index in 0..list.length {
        // SAFETY: the index is below the length.
        let item = unsafe { real(list, index) };
        if !item.exception.is_null() {
            return item;
        }
        product *= item.value;
    }
    ok(product)
}

/// Returns `ValueError` for two iterables of different lengths given to `sumprod`.
fn different_lengths<T>(value: T) -> Raising<T> {
    value_error(value, "Inputs are not the same length")
}

/// `math.sumprod(p, q)` of `int` or `bool` items: the exact sum of the products, an `int`.
///
/// # Safety
///
/// Both are live lists of `int` or `bool` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_sumprod_int(
    p: *const ListObject,
    q: *const ListObject,
) -> Raising<i64> {
    // SAFETY: the caller's guarantee.
    let (p, q) = unsafe { (&*p, &*q) };
    if p.length != q.length {
        return different_lengths(0);
    }
    let mut total = Integer::from_i128(0);
    for index in 0..p.length {
        // SAFETY: the index is below both lengths.
        let (x, y) = unsafe { (whole(p, index), whole(q, index)) };
        total = total.add(&x.multiply(&y));
    }
    Raising::ok(total.into_word())
}

/// A sum held as three doubles, which keeps the rounding errors of a sum of products.
#[derive(Clone, Copy)]
struct TripleLength {
    high: f64,
    low: f64,
    tiny: f64,
}

/// Returns `a + b` as a rounded sum and its error.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let x = a + b;
    let z = x - a;
    (x, (a - (x - z)) + (b - z))
}

impl TripleLength {
    const ZERO: Self = Self {
        high: 0.0,
        low: 0.0,
        tiny: 0.0,
    };

    /// Returns the sum with `x * y` added, as CPython's `tl_fma`.
    fn add_product(self, x: f64, y: f64) -> Self {
        let product = x * y;
        // SAFETY: `fma` has no preconditions.
        let product_error = unsafe { c::fma(x, y, -product) };
        let (high, high_error) = two_sum(self.high, product);
        let (low, low_error) = two_sum(self.low, product_error);
        let (low, carried) = two_sum(low, high_error);
        Self {
            high,
            low,
            tiny: self.tiny + low_error + carried,
        }
    }

    /// Returns the sum rounded to one double.
    fn rounded(self) -> f64 {
        let (high, low) = two_sum(self.low, self.high);
        self.tiny + low + high
    }
}

/// `math.sumprod(p, q)` with a `float` among the items: the products are summed in triple
/// length while the sum stays finite, then one by one as `float`s, as CPython does. The
/// result is a `float`; the caller gives the `int` 0 for empty iterables.
///
/// # Safety
///
/// Both are live lists of `int`, `bool` or `float` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_sumprod_float(
    p: *const ListObject,
    q: *const ListObject,
) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    let (p, q) = unsafe { (&*p, &*q) };
    let mut extended = Some(TripleLength::ZERO);
    let mut in_use = false;
    let mut total = 0.0_f64;
    for index in 0..p.length.min(q.length) {
        // SAFETY: the index is below both lengths.
        let (x, y) = unsafe { (real(p, index), real(q, index)) };
        if let Some(sum) = extended {
            if x.exception.is_null() && y.exception.is_null() {
                let next = sum.add_product(x.value, y.value);
                if next.high.is_finite() {
                    extended = Some(next);
                    in_use = true;
                    continue;
                }
            }
            // An overflow, or a value no `float` holds, ends the extended sum.
            extended = None;
            if in_use {
                total += sum.rounded();
            }
        }
        for each in [&x, &y] {
            if !each.exception.is_null() {
                return Raising::error(0.0, each.exception);
            }
        }
        total += x.value * y.value;
    }
    if p.length != q.length {
        return different_lengths(0.0);
    }
    if let Some(sum) = extended.filter(|_| in_use) {
        total += sum.rounded();
    }
    ok(total)
}
