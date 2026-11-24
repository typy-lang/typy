//! `KeyError` as the runtime raises it: the missing key as its one argument and the `repr` of
//! the key as its text, as CPython's `KeyError(key)` has, so `args` holds the key itself
//! (DAT-09, EXC-11).

use super::{ExceptionKind, ExceptionObject, raise, raise_with_message};
use crate::allocator::allocate;
use crate::list::{ItemKind, ItemType};
use crate::tuple::{Field, TupleType};

/// Returns a `KeyError` whose argument is `item` and whose text is its `repr`, as CPython
/// raises it for a missing key. When the `repr` raises, as for an `int` of more than 4300
/// digits, the text is the one CPython prints for an exception it cannot convert to text.
///
/// # Safety
///
/// `item` is a live item of `item_type`.
pub(crate) unsafe fn key_error(item_type: ItemType, item: u64) -> *mut ExceptionObject {
    // SAFETY: the caller passes a live item; the exception of a failed `repr` is owned here.
    unsafe {
        let text = item_type.repr(item);
        let exception = if text.exception.is_null() {
            raise_with_message(ExceptionKind::KeyError, text.value)
        } else {
            crate::refcount::typy_rt_decrement(text.exception.cast());
            raise(ExceptionKind::KeyError, "<exception str() failed>")
        };
        give_argument(exception, item_type, item);
        exception
    }
}

/// Returns a `KeyError` whose argument is the `str` `text`, such as `pop from an empty set`,
/// which its text shows quoted.
pub(crate) fn key_error_text(text: &str) -> *mut ExceptionObject {
    let mut quoted = crate::message::MessageBuffer::new();
    quoted.push_str("'");
    quoted.push_str(text);
    quoted.push_str("'");
    let exception = quoted.raise(ExceptionKind::KeyError);
    let argument = crate::string::from_str(text);
    // SAFETY: the exception is new and has no arguments; the string's reference passes to
    // the arguments, which retain it once more, so one is released here.
    unsafe {
        give_argument(
            exception,
            ItemType::from(ItemKind::Object),
            argument.expose_provenance() as u64,
        );
        crate::refcount::typy_rt_decrement(argument.cast());
    }
    exception
}

/// Gives `exception` the one argument `item`, with a reference of its own. The layout of the
/// arguments follows the item in the same block, so a key of any type, a tuple's included,
/// needs no descriptor of the compiler's, and the block is freed with the arguments.
///
/// # Safety
///
/// `exception` is live and has no arguments, and `item` is a live item of `item_type`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "malloc returns memory aligned for every type, and each part's offset is a multiple of eight"
)]
unsafe fn give_argument(exception: *mut ExceptionObject, item_type: ItemType, item: u64) {
    let data = item_type.words() * 8;
    let block = allocate(data + size_of::<Field>() + size_of::<TupleType>());
    // SAFETY: the block holds the item's words, then a field, then a layout, each at an
    // offset that is a multiple of eight; the caller passes a live item and exception.
    unsafe {
        item_type.store(block.cast::<u64>(), item);
        item_type.retain(item);
        let field = block.add(data).cast::<Field>();
        field.write(Field {
            kind: item_type.kind() as u32,
            offset: 0,
            tuple: item_type.descriptor(),
        });
        let layout = block.add(data + size_of::<Field>()).cast::<TupleType>();
        layout.write(TupleType {
            size: data as u64,
            length: 1,
            fields: field,
        });
        (*exception).arguments_type = layout;
        (*exception).arguments = block;
    }
}

#[cfg(test)]
mod tests;
