//! The keywords `print` takes from `**` dicts (EXP-22, ADR-0083): once the call has merged the
//! dicts, as it merges the keywords of any call, the values are checked as CPython's `print`
//! checks them, and each keyword reads its value from the merged dict or takes its default.

use crate::dict::DictObject;
use crate::exception::{ExceptionKind, ExceptionObject};
use crate::message::MessageBuffer;
use crate::string::StrObject;

/// The bit of the `usable` mask that says `sep` takes the dict's values.
const SEP: i64 = 1;
/// The bit that says `end` takes them.
const END: i64 = 2;
/// The bit that says `file` takes them.
const FILE: i64 = 4;

/// Returns the value of `name` in `merged`, the dict of the keywords `**` gave, with its item
/// type, or `None` when it has none.
///
/// # Safety
///
/// `merged` points to a live dict of `str` keys.
unsafe fn entry(merged: *const DictObject, name: &str) -> Option<(crate::list::ItemType, u64)> {
    let key = crate::string::from_str(name);
    // SAFETY: the caller's guarantee; the key is a live `str`, released after the search.
    unsafe {
        let found = entry_of(merged, key);
        crate::refcount::typy_rt_decrement(key.cast());
        found
    }
}

/// Returns the value of the key `key` in `merged`, with its item type.
///
/// # Safety
///
/// `merged` points to a live dict of `str` keys, and `key` to a live `str`.
unsafe fn entry_of(
    merged: *const DictObject,
    key: *const StrObject,
) -> Option<(crate::list::ItemType, u64)> {
    let bits = key.expose_provenance() as u64;
    // SAFETY: the caller's guarantee.
    unsafe {
        crate::dict::find(merged, bits, (*merged).key_type().hash(bits))
            .map(|position| ((*merged).value_type(), (*merged).value(position)))
    }
}

/// Returns the error `print` raises for a keyword `**` gave whose value it cannot take: a
/// `sep` or an `end` that is no `str` or `None`, `TypeError`, then a `file` without `write`,
/// `AttributeError`; null when every value fits. `usable`, an `int` word, has a bit set for
/// each keyword whose type the dict's values have.
///
/// # Safety
///
/// `merged` points to a live dict of `str` keys.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_print_check(
    merged: *const DictObject,
    usable: i64,
) -> *mut ExceptionObject {
    let usable = crate::word::inline_value(usable);
    for (name, bit) in [("sep", SEP), ("end", END)] {
        // SAFETY: the caller's guarantee.
        if let Some((ty, value)) = unsafe { entry(merged, name) }
            && usable & bit == 0
        {
            let mut message = MessageBuffer::new();
            message.push_str(name);
            message.push_str(" must be None or a string, not ");
            // SAFETY: the value is a live item of the dict's value type.
            message.push_str(unsafe { ty.class_name(value) });
            return message.raise(ExceptionKind::TypeError);
        }
    }
    // SAFETY: the caller's guarantee.
    if let Some((ty, value)) = unsafe { entry(merged, "file") }
        && usable & FILE == 0
    {
        let mut message = MessageBuffer::new();
        message.push_str("'");
        // SAFETY: the value is a live item of the dict's value type.
        message.push_str(unsafe { ty.class_name(value) });
        message.push_str("' object has no attribute 'write'");
        return message.raise(ExceptionKind::AttributeError);
    }
    core::ptr::null_mut()
}

/// Returns the `str` the keyword `name` takes from `merged`, borrowed: its value, or `default`
/// when the dict has none or holds `None` for it.
///
/// # Safety
///
/// `merged` points to a live dict of `str` keys whose values are `str` or `None`, and `name`
/// and `default` to live `str` objects.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_print_text(
    merged: *const DictObject,
    name: *const StrObject,
    default: *const StrObject,
) -> *const StrObject {
    // SAFETY: the caller's guarantee.
    unsafe {
        match entry_of(merged, name).and_then(|(ty, value)| ty.held_value(value)) {
            Some((_, text)) => {
                core::ptr::with_exposed_provenance(usize::try_from(text).unwrap_or(0))
            }
            None => default,
        }
    }
}

/// Returns the text file `print` writes to, borrowed: the `file` of `merged`, or standard
/// output when the dict has none or holds `None` for it.
///
/// # Safety
///
/// `merged` points to a live dict of `str` keys whose values are text files or `None`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_print_file(
    merged: *const DictObject,
) -> *const crate::text_file::TextFile {
    // SAFETY: the caller's guarantee.
    unsafe {
        match entry(merged, "file").and_then(|(ty, value)| ty.held_value(value)) {
            Some((_, file)) => {
                core::ptr::with_exposed_provenance(usize::try_from(file).unwrap_or(0))
            }
            None => crate::text_file::typy_rt_sys_stdout(),
        }
    }
}

/// Returns whether `print` flushes: the truth of the `flush` of `merged`, false when it has
/// none.
///
/// # Safety
///
/// `merged` points to a live dict of `str` keys.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_print_flush(merged: *const DictObject) -> bool {
    // SAFETY: the caller's guarantee; the value is a live item of the dict's value type.
    unsafe { entry(merged, "flush").is_some_and(|(ty, value)| ty.is_true(value)) }
}

#[cfg(test)]
mod tests;
