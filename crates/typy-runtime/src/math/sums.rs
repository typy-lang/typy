//! The functions of `math` over iterables of numbers: `fsum`, `dist`, `prod` and `sumprod`,
//! each given lists of `int`, `bool` or `float` items and computed as CPython computes them
//! (LIB-12).
//!
//! Derived from CPython's `Modules/mathmodule.c` under the PSF License Agreement, which
//! THIRD-PARTY-NOTICES.md holds with a summary of the changes.
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

/// An item of a list of numbers: an `int` or a `bool` as an integer, with whether it is a
/// `bool`, or a `float`.
enum Number {
    Whole(Integer, bool),
    Real(f64),
}

/// Returns the kind of item `index` of a list of numbers: the item's own, or for a union of
/// number types the member it holds (LIB-15).
///
/// # Safety
///
/// `list` is a live list of `int`, `bool` or `float` items, or of a union of them, and
/// `index` is below its length.
unsafe fn held(list: &ListObject, index: usize) -> (ItemKind, u64) {
    // SAFETY: the caller keeps the index below the length; a number is never `None`.
    unsafe {
        let item = list.item(index);
        list.item_type()
            .held_value(item)
            .map_or((ItemKind::Int, item), |(ty, value)| (ty.kind(), value))
    }
}

/// Returns item `index` of a list of numbers.
///
/// # Safety
///
/// As `held`.
unsafe fn number(list: &ListObject, index: usize) -> Number {
    // SAFETY: the caller's guarantee.
    let (kind, bits) = unsafe { held(list, index) };
    match kind {
        ItemKind::Float => Number::Real(f64::from_bits(bits)),
        ItemKind::Bool => Number::Whole(Integer::from_i128(i128::from(bits != 0)), true),
        #[allow(clippy::cast_possible_wrap, reason = "an `int` item is an `int` word")]
        // SAFETY: the list holds the large integer of its item.
        _ => Number::Whole(unsafe { integer(bits as i64) }, false),
    }
}

impl Number {
    /// Returns the number as a `float`, as CPython converts it: an `int` too large for one
    /// raises `OverflowError`.
    fn real(&self) -> Raising<f64> {
        match self {
            Self::Real(value) => ok(*value),
            Self::Whole(value, _) => crate::float::integer_to_float(value),
        }
    }
}

/// Returns item `index` of a list of numbers as a `float`, as CPython converts it.
///
/// # Safety
///
/// As `held`.
unsafe fn real(list: &ListObject, index: usize) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    unsafe { number(list, index) }.real()
}

/// Returns item `index` of a list of `int` or `bool` items, or of a union of them, as an
/// integer.
///
/// # Safety
///
/// As `held`, for a list with no `float` item.
unsafe fn whole(list: &ListObject, index: usize) -> Integer {
    // SAFETY: the caller's guarantee.
    match unsafe { number(list, index) } {
        Number::Whole(value, _) => value,
        Number::Real(_) => Integer::from_i128(0),
    }
}

/// Returns true when a list of numbers holds a `float`, which makes the result of `prod` or
/// `sumprod` a `float` (LIB-15).
///
/// # Safety
///
/// `values` is a live list of `int`, `bool` or `float` items, or of a union of them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_math_has_float(values: *const ListObject) -> bool {
    // SAFETY: the caller's guarantee.
    let list = unsafe { &*values };
    (0..list.length).any(|index| {
        // SAFETY: the index is below the length.
        unsafe { held(list, index) }.0 == ItemKind::Float
    })
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

crate::export_float!(unsafe typy_rt_math_fsum = math_fsum(values: *const ListObject));

/// `math.fsum(values)`: the sum, rounded once, by Shewchuk's partial sums as CPython keeps
/// them.
///
/// # Safety
///
/// `values` is a live list of `int`, `bool` or `float` items.
pub(crate) unsafe fn math_fsum(values: *const ListObject) -> Raising<f64> {
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

crate::export_float!(unsafe typy_rt_math_dist = math_dist(p: *const ListObject, q: *const ListObject));

/// `math.dist(p, q)`: the Euclidean distance between two points of as many coordinates.
///
/// # Safety
///
/// Both are live lists of `int`, `bool` or `float` items.
pub(crate) unsafe fn math_dist(p: *const ListObject, q: *const ListObject) -> Raising<f64> {
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

crate::export_float!(unsafe typy_rt_math_prod_float = math_prod_float(values: *const ListObject, start: f64));

/// `math.prod(values, start=start)` of at least one item with a `float` among the items or
/// the start, given as a `float`: each item multiplies the product as a `float`.
///
/// # Safety
///
/// `values` is a live list of `int`, `bool` or `float` items.
pub(crate) unsafe fn math_prod_float(values: *const ListObject, start: f64) -> Raising<f64> {
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

crate::export_float!(unsafe typy_rt_math_prod_mixed = math_prod_mixed(values: *const ListObject, start: i64));

/// `math.prod(values, start=start)` of an `int` start and items with a `float` among them:
/// the exact product of the integers up to the first `float`, then each item multiplies the
/// product as a `float`, as CPython's `math.prod` does.
///
/// # Safety
///
/// `values` is a live list of `int`, `bool` or `float` items, or of a union of them, and
/// `start` an `int` word.
pub(crate) unsafe fn math_prod_mixed(values: *const ListObject, start: i64) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    let (list, mut exact) = unsafe { (&*values, Some(integer(start))) };
    let mut product = 0.0_f64;
    for index in 0..list.length {
        // SAFETY: the index is below the length.
        let item = unsafe { number(list, index) };
        let factor = match (exact.take(), item) {
            (Some(so_far), Number::Whole(value, _)) => {
                exact = Some(so_far.multiply(&value));
                continue;
            }
            (Some(so_far), Number::Real(value)) => {
                let converted = crate::float::integer_to_float(&so_far);
                if !converted.exception.is_null() {
                    return converted;
                }
                product = converted.value;
                value
            }
            (None, item) => {
                let converted = item.real();
                if !converted.exception.is_null() {
                    return converted;
                }
                converted.value
            }
        };
        product *= factor;
    }
    match exact {
        Some(whole) => crate::float::integer_to_float(&whole),
        None => ok(product),
    }
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

/// The running total of `sumprod` past its fast paths: an exact integer until a `float`
/// joins it.
enum Total {
    Whole(Integer),
    Real(f64),
}

impl Total {
    /// Returns `self + other`, as CPython adds an `int` and a `float`: the `int` converted,
    /// which raises `OverflowError` when it is too large.
    fn plus(self, other: Self) -> Raising<Self> {
        let (left, right) = match (self, other) {
            (Self::Whole(left), Self::Whole(right)) => {
                return Raising::ok(Self::Whole(left.add(&right)));
            }
            (Self::Real(left), Self::Real(right)) => return Raising::ok(Self::Real(left + right)),
            (Self::Whole(whole), Self::Real(real)) | (Self::Real(real), Self::Whole(whole)) => {
                (crate::float::integer_to_float(&whole), real)
            }
        };
        if left.exception.is_null() {
            Raising::ok(Self::Real(left.value + right))
        } else {
            Raising::error(Self::Real(0.0), left.exception)
        }
    }
}

/// Returns `p * q` of two numbers, as CPython multiplies them.
fn product(p: &Number, q: &Number) -> Raising<Total> {
    if let (Number::Whole(p, _), Number::Whole(q, _)) = (p, q) {
        return Raising::ok(Total::Whole(p.multiply(q)));
    }
    let (p, q) = (p.real(), q.real());
    for each in [&p, &q] {
        if !each.exception.is_null() {
            return Raising::error(Total::Real(0.0), each.exception);
        }
    }
    Raising::ok(Total::Real(p.value * q.value))
}

/// Returns a number as a machine integer, as `PyLong_AsLongAndOverflow` gives an exact `int`,
/// or `None` for a `bool`, a `float`, or an `int` too large.
fn small(number: &Number) -> Option<i64> {
    match number {
        Number::Whole(value, false) => value.to_i128().and_then(|value| i64::try_from(value).ok()),
        _ => None,
    }
}

/// Returns the pair as two doubles for the extended sum: two `float`s, or a `float` with an
/// `int` or a `bool` that converts, or `None`.
fn reals(p: &Number, q: &Number) -> Option<(f64, f64)> {
    match (p, q) {
        (Number::Real(p), Number::Real(q)) => Some((*p, *q)),
        (Number::Real(real), whole) | (whole, Number::Real(real)) => {
            let converted = whole.real();
            if !converted.exception.is_null() {
                // CPython clears this error and takes the pair the slow way, which raises it
                // again.
                // SAFETY: the exception is new and owned here.
                unsafe { crate::refcount::typy_rt_decrement(converted.exception.cast()) };
                return None;
            }
            // The order of the factors does not change their product.
            Some((*real, converted.value))
        }
        _ => None,
    }
}

crate::export_float!(unsafe typy_rt_math_sumprod_float = math_sumprod_float(p: *const ListObject, q: *const ListObject));

/// `math.sumprod(p, q)` with a `float` among the items, as CPython's `math_sumprod_impl`
/// computes it: pairs of exact `int`s summed in a machine integer while it does not
/// overflow, then pairs with a `float` summed in triple length while the sum stays finite,
/// each path closing for good at the first pair it cannot take and adding its sum to the
/// total; every pair after that adds its product to the total, an exact integer until a
/// `float` joins it. The caller gives the `int` 0 for empty iterables.
///
/// # Safety
///
/// Both are live lists of `int`, `bool` or `float` items, or of a union of them.
pub(crate) unsafe fn math_sumprod_float(
    p: *const ListObject,
    q: *const ListObject,
) -> Raising<f64> {
    // SAFETY: the caller's guarantee.
    let (p, q) = unsafe { (&*p, &*q) };
    let mut total = Total::Whole(Integer::from_i128(0));
    let mut whole_path = Some(0_i64);
    let mut real_path = Some(TripleLength::ZERO);
    let mut real_in_use = false;
    let common = p.length.min(q.length);
    for index in 0..=common {
        let finished = index == common;
        // SAFETY: an index below the common length is below both lengths.
        let pair = (!finished).then(|| unsafe { (number(p, index), number(q, index)) });
        if let Some(sum) = whole_path {
            let next = pair
                .as_ref()
                .and_then(|(x, y)| small(x)?.checked_mul(small(y)?)?.checked_add(sum));
            if let Some(next) = next {
                whole_path = Some(next);
                continue;
            }
            whole_path = None;
            // The exact path runs first, so the total is still 0 and becomes its sum.
            total = Total::Whole(Integer::from_i128(i128::from(sum)));
        }
        if let Some(sum) = real_path {
            let next = pair
                .as_ref()
                .and_then(|(x, y)| reals(x, y))
                .map(|(x, y)| sum.add_product(x, y))
                .filter(|next| next.high.is_finite());
            if let Some(next) = next {
                real_path = Some(next);
                real_in_use = true;
                continue;
            }
            real_path = None;
            if real_in_use {
                let added = total.plus(Total::Real(sum.rounded()));
                if !added.exception.is_null() {
                    return Raising::error(0.0, added.exception);
                }
                total = added.value;
            }
        }
        let Some((x, y)) = pair else {
            break;
        };
        let term = product(&x, &y);
        if !term.exception.is_null() {
            return Raising::error(0.0, term.exception);
        }
        let added = total.plus(term.value);
        if !added.exception.is_null() {
            return Raising::error(0.0, added.exception);
        }
        total = added.value;
    }
    if p.length != q.length {
        return different_lengths(0.0);
    }
    match total {
        Total::Real(value) => ok(value),
        Total::Whole(value) => crate::float::integer_to_float(&value),
    }
}
