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
mod tests {
    use super::{key_error, key_error_text};
    use crate::list::{ItemKind, ItemType};
    use crate::refcount::typy_rt_decrement;
    use crate::string::{StrObject, from_str};
    use crate::tuple::{Field, TupleType};
    use crate::word::inline;

    /// Returns the text of a string, and releases it.
    fn taken(string: *mut StrObject) -> String {
        // SAFETY: the string is live and owned here.
        unsafe {
            let text = (0..crate::string::length(string))
                .map(|index| char::from_u32(crate::string::code_point(string, index)).unwrap())
                .collect();
            typy_rt_decrement(string.cast());
            text
        }
    }

    /// Returns `str` and `repr` of an exception, and releases it.
    fn shown(exception: *mut super::ExceptionObject) -> (String, String) {
        // SAFETY: the exception is live and owned here.
        unsafe {
            let text = crate::exception::typy_rt_exception_text(exception, inline(0).unwrap());
            let repr = crate::exception::typy_rt_exception_text(exception, inline(1).unwrap());
            typy_rt_decrement(exception.cast());
            (taken(text.value), taken(repr.value))
        }
    }

    #[test]
    fn a_missing_string_key_is_the_argument_and_its_repr_the_text() {
        let key = from_str("k");
        // SAFETY: the key is live.
        let error = unsafe {
            key_error(
                ItemType::from(ItemKind::Object),
                key.expose_provenance() as u64,
            )
        };
        assert_eq!(shown(error), ("'k'".to_owned(), "KeyError('k')".to_owned()));
        // SAFETY: the key is live; the exception gave back the reference it took.
        unsafe {
            assert_eq!((*key).header.reference_count, 1);
            typy_rt_decrement(key.cast());
        }
    }

    #[test]
    fn a_missing_tuple_key_is_kept_whole_as_the_argument() {
        let fields = Box::leak(Box::new([
            Field::plain(ItemKind::Object, 0),
            Field::plain(ItemKind::Int, 8),
        ]));
        let pair = Box::leak(Box::new(TupleType {
            size: 16,
            length: 2,
            fields: fields.as_ptr(),
        }));
        let name = from_str("a");
        let words = [
            name.expose_provenance() as u64,
            inline(1).unwrap().cast_unsigned(),
        ];
        // SAFETY: the words are a live tuple of the descriptor.
        let error = unsafe {
            key_error(
                ItemType::tuple(pair),
                words.as_ptr().expose_provenance() as u64,
            )
        };
        assert_eq!(
            shown(error),
            ("('a', 1)".to_owned(), "KeyError(('a', 1))".to_owned())
        );
        // SAFETY: the string is live; the exception gave back the reference it took.
        unsafe {
            assert_eq!((*name).header.reference_count, 1);
            typy_rt_decrement(name.cast());
        }
    }

    #[test]
    fn a_key_error_of_a_text_shows_it_quoted_and_keeps_it_plain_as_the_argument() {
        assert_eq!(
            shown(key_error_text("pop from an empty set")),
            (
                "'pop from an empty set'".to_owned(),
                "KeyError('pop from an empty set')".to_owned()
            )
        );
    }
}
