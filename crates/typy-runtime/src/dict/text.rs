//! Writes a dict and its views as text, as CPython's `repr` does (DAT-09).

use super::DictObject;
use crate::Raising;
use crate::list::ItemType;
use crate::string::StrBuilder;
use crate::string::StrObject;

/// Adds an ASCII text to a string being built.
fn push_ascii(builder: &mut StrBuilder, text: &str) {
    text.bytes().for_each(|byte| builder.push(u32::from(byte)));
}

/// Adds `repr(item)` to a string being built; returns the exception when it raises, which
/// only an `int` of more than 4300 digits does.
///
/// # Safety
///
/// `item` is a live item of `item_type`.
unsafe fn push_repr(
    builder: &mut StrBuilder,
    item_type: ItemType,
    item: u64,
) -> Result<(), *mut crate::exception::ExceptionObject> {
    // SAFETY: the caller's guarantee; the piece is released once it is copied.
    unsafe {
        let piece = item_type.repr(item);
        if !piece.exception.is_null() {
            return Err(piece.exception);
        }
        builder.push_str(piece.value);
        crate::refcount::typy_rt_decrement(piece.value.cast());
        Ok(())
    }
}

/// What a dict's text shows of each entry.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shown {
    /// `key: value`, for the dict.
    Entry,
    /// The key, for `dict_keys`.
    Key,
    /// The value, for `dict_values`.
    Value,
    /// `(key, value)`, for `dict_items`.
    Pair,
}

/// Returns the text of the entries between `open` and `close`, each shown as `shown` says.
///
/// # Safety
///
/// `dict` points to a live dict.
unsafe fn entries_text(
    dict: &DictObject,
    shown: Shown,
    open: &str,
    close: &str,
) -> Raising<*mut StrObject> {
    let (key_type, value_type) = (dict.key_type(), dict.value_type());
    let mut builder = StrBuilder::with_capacity(open.len() + close.len());
    push_ascii(&mut builder, open);
    // SAFETY: the caller passes a live dict; each live entry holds live items.
    let written = unsafe {
        dict.live_entries()
            .enumerate()
            .try_for_each(|(count, index)| {
                if count > 0 {
                    push_ascii(&mut builder, ", ");
                }
                let (key, value) = (dict.key(index), dict.value(index));
                match shown {
                    Shown::Key => push_repr(&mut builder, key_type, key),
                    Shown::Value => push_repr(&mut builder, value_type, value),
                    Shown::Entry => {
                        push_repr(&mut builder, key_type, key)?;
                        push_ascii(&mut builder, ": ");
                        push_repr(&mut builder, value_type, value)
                    }
                    Shown::Pair => {
                        push_ascii(&mut builder, "(");
                        push_repr(&mut builder, key_type, key)?;
                        push_ascii(&mut builder, ", ");
                        push_repr(&mut builder, value_type, value)?;
                        push_ascii(&mut builder, ")");
                        Ok(())
                    }
                }
            })
    };
    match written {
        Ok(()) => {
            push_ascii(&mut builder, close);
            Raising::ok(builder.finish())
        }
        Err(exception) => {
            builder.discard();
            Raising::error(core::ptr::null_mut(), exception)
        }
    }
}

/// Returns `repr(dict)`: `{key: value, ...}` in insertion order, or `{}`.
///
/// # Safety
///
/// `dict` points to a live dict.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_repr(dict: *const DictObject) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    unsafe { entries_text(&*dict, Shown::Entry, "{", "}") }
}

/// Returns the text of a view of the dict whose kind is the `int` word `view`: 0 for
/// `dict_keys([...])`, 1 for `dict_values([...])` and 2 for `dict_items([(key, value), ...])`.
///
/// # Safety
///
/// `dict` points to a live dict.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_dict_view_repr(
    dict: *const DictObject,
    view: i64,
) -> Raising<*mut StrObject> {
    let (shown, open) = match crate::word::inline_value(view) {
        0 => (Shown::Key, "dict_keys(["),
        1 => (Shown::Value, "dict_values(["),
        _ => (Shown::Pair, "dict_items(["),
    };
    // SAFETY: the caller's guarantee.
    unsafe { entries_text(&*dict, shown, open, "])") }
}
