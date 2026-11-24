//! The arguments of an exception, `args`, and the code of a `SystemExit`, `code`, as values a
//! program prints, compares and tests (EXC-11, EXC-13, ADR-0030). Each is read from the
//! exception itself, which the program holds in its place.
//!
//! An exception the runtime raised holds its text, which stands for its one argument.

use super::ExceptionObject;
use super::text::Arguments;
use crate::Raising;
use crate::list::{ItemKind, ItemType};
use crate::string::{StrObject, from_str};
use crate::tuple::{TupleType, field, field_type};

/// The part `args`.
pub(super) const ARGUMENTS: i64 = 0;

/// Returns the part an `int` word names, 0 for `args` and 1 for `code`.
fn decoded(part: i64) -> i64 {
    crate::word::inline_value(part)
}

/// What a part of an exception holds.
pub(super) enum Part {
    /// No value: the `code` of a `SystemExit` without arguments, `None`.
    Nothing,
    /// One value of this type: the runtime's text, or the one argument of a `SystemExit`.
    One(ItemType, u64),
    /// The tuple of these arguments.
    Tuple(Arguments),
    /// `args` of no arguments, `()`.
    Empty,
}

/// Returns what `part`, 0 for `args` or 1 for `code`, of `exception` holds: its arguments
/// first, then the text the runtime gave it, which stands for its one argument.
pub(super) fn part_of(exception: &ExceptionObject, part: i64) -> Part {
    let arguments = Arguments::of(exception);
    if arguments.is_none() && !exception.message.is_null() {
        let text = ItemType::from(ItemKind::Object);
        return Part::One(text, exception.message.expose_provenance() as u64);
    }
    match (arguments, part) {
        (None, ARGUMENTS) => Part::Empty,
        (None, _) => Part::Nothing,
        // `code` is the one argument itself.
        (Some(arguments), _) if part != ARGUMENTS && arguments.count() == 1 => {
            // SAFETY: the arguments are live and laid out as their descriptor says; an
            // optional or a union is read as the value it holds.
            let held = unsafe {
                field_type(arguments.layout(), 0).held_value(field(
                    arguments.layout(),
                    arguments.address,
                    0,
                ))
            };
            match held {
                Some((ty, value)) => Part::One(ty, value),
                None => Part::Nothing,
            }
        }
        (Some(arguments), _) => Part::Tuple(arguments),
    }
}

/// The layout of `args` of an exception the runtime raised with a text: one `str`, the
/// text, read from the field that holds it.
static TEXT_ARGUMENTS: TupleType = TupleType {
    size: 8,
    length: 1,
    fields: &raw const TEXT_FIELD,
};

static TEXT_FIELD: crate::tuple::Field = crate::tuple::Field::plain(ItemKind::Object, 0);

/// Returns true when `args` of `exception` is laid out as `layout`, the layout of one
/// member of the union of tuples `args` has in the program: its arguments as `args` shows
/// them, the runtime's text as one `str`, or no items (EXC-15, ADR-0034). When `raw` is the
/// `int` word of 1, the arguments are compared as the exception holds them, file names
/// included, which the attributes of an `OSError` read (ADR-0035).
///
/// # Safety
///
/// `exception` points to a live exception, and `layout` to a descriptor the compiler emitted.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_arguments_match(
    exception: *const ExceptionObject,
    layout: *const TupleType,
    raw: i64,
) -> bool {
    // SAFETY: the caller's guarantee.
    unsafe {
        let exception = self::exception(exception);
        if decoded(raw) != 0 && !exception.arguments.is_null() {
            return crate::tuple::same_layout(exception.arguments_type, layout);
        }
        match Arguments::of(exception) {
            Some(arguments) => crate::tuple::same_layout(arguments.layout(), layout),
            None if !exception.message.is_null() => {
                crate::tuple::same_layout(&raw const TEXT_ARGUMENTS, layout)
            }
            None => (*layout).length == 0,
        }
    }
}

/// Returns the address of the items of `args` of `exception`, borrowed from it: its
/// arguments, whose first two an `OSError` with file names shows, the field that holds the
/// runtime's text, or 0 for none.
///
/// # Safety
///
/// `exception` points to a live exception.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_arguments_address(
    exception: *const ExceptionObject,
) -> u64 {
    // SAFETY: the caller's guarantee.
    let exception = unsafe { self::exception(exception) };
    if !exception.arguments.is_null() {
        exception.arguments.expose_provenance() as u64
    } else if exception.message.is_null() {
        0
    } else {
        (&raw const exception.message).expose_provenance() as u64
    }
}

/// Returns the exception a part is read from.
///
/// # Safety
///
/// `object` points to a live exception.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "an exception comes from malloc, aligned for every type"
)]
unsafe fn exception<'a>(object: *const ExceptionObject) -> &'a ExceptionObject {
    // SAFETY: the caller's guarantee.
    unsafe { &*object }
}

/// Returns `str` of a part, or `repr` when `repr` is 1: the tuple of `args`, as a tuple
/// writes itself, and `code` as its value does.
///
/// # Safety
///
/// `object` points to a live exception, and `part` is the `int` word of 0 for `args` or 1 for
/// `code`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_part_text(
    object: *const ExceptionObject,
    part: i64,
    repr: i64,
) -> Raising<*mut StrObject> {
    let (part, repr) = (decoded(part), decoded(repr));
    // SAFETY: the caller's guarantee; each value is live.
    unsafe {
        match part_of(exception(object), part) {
            Part::Nothing => Raising::ok(from_str("None")),
            Part::Empty => Raising::ok(from_str("()")),
            // `args` of the runtime's text is a tuple of one.
            Part::One(ty, value) if part == ARGUMENTS => {
                let inner = ty.repr(value);
                if !inner.exception.is_null() {
                    return inner;
                }
                let (open, close) = (from_str("("), from_str(",)"));
                let head = crate::string::typy_rt_str_concat(open, inner.value);
                let whole = crate::string::typy_rt_str_concat(head, close);
                for text in [open, inner.value, head, close] {
                    crate::refcount::typy_rt_decrement(text.cast());
                }
                Raising::ok(whole)
            }
            Part::One(ty, value) if repr == 1 => ty.repr(value),
            Part::One(ty, value) => ty.str(value),
            Part::Tuple(arguments) => crate::tuple::repr(arguments.layout(), arguments.address),
        }
    }
}

/// Returns `len(exception.args)`, an `int` word.
///
/// # Safety
///
/// `object` points to a live exception.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_args_length(object: *const ExceptionObject) -> i64 {
    // SAFETY: the caller's guarantee.
    let count = match part_of(unsafe { exception(object) }, ARGUMENTS) {
        Part::Nothing | Part::Empty => 0,
        Part::One(..) => 1,
        Part::Tuple(arguments) => arguments.count(),
    };
    crate::word::inline(i64::try_from(count).unwrap_or(i64::MAX)).unwrap_or(1)
}

/// Returns whether a part is true: `args` when it holds an argument, and `code` as its value
/// tests.
///
/// # Safety
///
/// `object` points to a live exception, and `part` is the `int` word of 0 for `args` or 1 for
/// `code`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_part_truth(
    object: *const ExceptionObject,
    part: i64,
) -> bool {
    let part = decoded(part);
    // SAFETY: the caller's guarantee; each value is live.
    unsafe {
        match part_of(exception(object), part) {
            Part::Nothing | Part::Empty => false,
            Part::One(_, _) if part == ARGUMENTS => true,
            Part::One(ty, value) => ty.is_true(value),
            Part::Tuple(arguments) => arguments.count() > 0,
        }
    }
}

/// Returns whether a part equals the one field of the tuple `other` of layout `other_type`:
/// `args` a tuple, and `code` its value, as `==` compares values of two types.
///
/// # Safety
///
/// `object` points to a live exception, `part` is the `int` word of 0 for `args` or 1 for
/// `code`, and `other`
/// is the address of a live tuple of one field laid out as `other_type` says.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_exception_part_equal(
    object: *const ExceptionObject,
    part: i64,
    other_type: *const TupleType,
    other: u64,
) -> bool {
    let part = decoded(part);
    // SAFETY: the caller's guarantee; each value is live.
    unsafe {
        // An optional or a union is compared as the value it holds.
        let held = field_type(other_type, 0).held_value(field(other_type, other, 0));
        let held = held.filter(|(ty, _)| ty.kind() != ItemKind::NoneValue);
        let Some((wanted_type, wanted)) = held else {
            return matches!(part_of(exception(object), part), Part::Nothing);
        };
        let empty =
            || wanted_type.kind() == ItemKind::Tuple && (*wanted_type.descriptor()).length == 0;
        match part_of(exception(object), part) {
            Part::Nothing => false,
            Part::Empty => empty(),
            Part::One(ty, value) if part == ARGUMENTS => {
                wanted_type.kind() == ItemKind::Tuple
                    && (*wanted_type.descriptor()).length == 1
                    && crate::list::values_equal(
                        ty,
                        value,
                        field_type(wanted_type.descriptor(), 0),
                        field(wanted_type.descriptor(), wanted, 0),
                    )
            }
            Part::One(ty, value) => crate::list::values_equal(ty, value, wanted_type, wanted),
            Part::Tuple(arguments) => crate::list::values_equal(
                ItemType::tuple(arguments.layout()),
                arguments.address,
                wanted_type,
                wanted,
            ),
        }
    }
}

#[cfg(test)]
mod tests;
