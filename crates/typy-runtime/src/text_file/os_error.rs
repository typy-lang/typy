//! `OSError` and its subclasses for an error number, as CPython raises them (ADR-0019), and
//! as a program builds them with two to five arguments, with their attributes `errno`,
//! `strerror`, `filename` and `filename2` (EXC-14).

#[cfg(target_os = "linux")]
use super::errno_text::ERRNO_TEXTS;
use crate::Raising;
use crate::exception::ExceptionObject;
use crate::exception::classes::{
    BLOCKING_IO_ERROR, BROKEN_PIPE_ERROR, CHILD_PROCESS_ERROR, CONNECTION_ABORTED_ERROR,
    CONNECTION_REFUSED_ERROR, CONNECTION_RESET_ERROR, FILE_EXISTS_ERROR, FILE_NOT_FOUND_ERROR,
    INTERRUPTED_ERROR, IS_A_DIRECTORY_ERROR, NOT_A_DIRECTORY_ERROR, OS_ERROR, PERMISSION_ERROR,
    PROCESS_LOOKUP_ERROR, TIMEOUT_ERROR,
};
use crate::header::ObjectHeader;
use crate::list::ItemKind;
use crate::message::MessageBuffer;
use crate::string::StrObject;
use crate::tuple::{Field, TupleType};
use crate::type_object::TypeObject;

/// The layout of the arguments of an `OSError` the runtime raises: `errno` and `strerror`.
static ERROR_ARGUMENTS: TupleType = TupleType {
    size: 16,
    length: 2,
    fields: ERROR_FIELDS.as_ptr(),
};

static ERROR_FIELDS: [Field; 3] = [
    Field::plain(ItemKind::Int, 0),
    Field::plain(ItemKind::Object, 8),
    Field::plain(ItemKind::Object, 16),
];

/// The layout of the arguments of an `OSError` the runtime raises for a file: `errno`,
/// `strerror` and `filename`.
static NAMED_ERROR_ARGUMENTS: TupleType = TupleType {
    size: 24,
    length: 3,
    fields: ERROR_FIELDS.as_ptr(),
};

/// The layout of the arguments of an `OSError` the runtime raises for a file opened from a
/// descriptor: `errno`, `strerror` and the descriptor, CPython's file name for it.
static DESCRIPTOR_ERROR_ARGUMENTS: TupleType = TupleType {
    size: 24,
    length: 3,
    fields: DESCRIPTOR_FIELDS.as_ptr(),
};

static DESCRIPTOR_FIELDS: [Field; 3] = [
    Field::plain(ItemKind::Int, 0),
    Field::plain(ItemKind::Object, 8),
    Field::plain(ItemKind::Int, 16),
];

/// Returns the class CPython gives an error number on the system the program runs on: the
/// subclass of `OSError` its `errno` table names, or `OSError` itself (EXC-19). The numbers
/// below 35 are the same on every system.
pub(crate) fn class_of(errno: i32) -> &'static TypeObject {
    use crate::libc::{
        EAGAIN, EALREADY, ECONNABORTED, ECONNREFUSED, ECONNRESET, EINPROGRESS, ESHUTDOWN, ETIMEDOUT,
    };
    match errno {
        1 | 13 => &PERMISSION_ERROR,
        2 => &FILE_NOT_FOUND_ERROR,
        3 => &PROCESS_LOOKUP_ERROR,
        4 => &INTERRUPTED_ERROR,
        10 => &CHILD_PROCESS_ERROR,
        EAGAIN | EALREADY | EINPROGRESS => &BLOCKING_IO_ERROR,
        17 => &FILE_EXISTS_ERROR,
        20 => &NOT_A_DIRECTORY_ERROR,
        21 => &IS_A_DIRECTORY_ERROR,
        32 | ESHUTDOWN => &BROKEN_PIPE_ERROR,
        ECONNABORTED => &CONNECTION_ABORTED_ERROR,
        ECONNRESET => &CONNECTION_RESET_ERROR,
        ETIMEDOUT => &TIMEOUT_ERROR,
        ECONNREFUSED => &CONNECTION_REFUSED_ERROR,
        _ => &OS_ERROR,
    }
}

/// Returns the text the system's `strerror` gives an error number, as CPython shows it on
/// macOS (EXC-19, ADR-0057).
#[cfg(target_os = "macos")]
pub(crate) fn text_of(errno: i32) -> *mut StrObject {
    let mut text = core::ptr::null_mut();
    crate::libc::error_text(errno, |bytes| {
        text = crate::string::from_str(core::str::from_utf8(bytes).unwrap_or("Unknown error"));
    });
    text
}

/// Returns glibc's text of an error number, `Unknown error 200` for one it does not name, as
/// CPython shows it on Linux whichever C library the program links (ADR-0019).
#[cfg(target_os = "linux")]
pub(crate) fn text_of(errno: i32) -> *mut StrObject {
    let known = usize::try_from(errno)
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|index| ERRNO_TEXTS.get(index));
    if let Some(text) = known {
        return crate::string::from_str(text);
    }
    let mut text = MessageBuffer::new();
    text.push_str("Unknown error ");
    text.push_number(i64::from(errno));
    text.text()
}

/// Returns a new exception of the class for `errno` about a file descriptor, written as
/// `[Errno 21] Is a directory: 3`, and holding the arguments `(errno, strerror, descriptor)`,
/// of which `args` shows the first two and `filename` is the descriptor.
pub(crate) fn os_error_of_descriptor(errno: i32, descriptor: i32) -> *mut ExceptionObject {
    let strerror = text_of(errno);
    let mut message = MessageBuffer::new();
    message.push_str("[Errno ");
    message.push_number(i64::from(errno));
    message.push_str("] ");
    // SAFETY: the text is a live string.
    unsafe { message.push_text(strerror) };
    message.push_str(": ");
    message.push_number(i64::from(descriptor));
    let exception = crate::exception::raise_class(class_of(errno), message.text());
    let number = crate::word::inline(i64::from(errno))
        .unwrap_or(1)
        .cast_unsigned();
    let descriptor = crate::word::inline(i64::from(descriptor))
        .unwrap_or(1)
        .cast_unsigned();
    // SAFETY: the exception is new and takes the reference to the text.
    unsafe {
        let text = strerror.expose_provenance() as u64;
        crate::exception::set_arguments(
            exception,
            &DESCRIPTOR_ERROR_ARGUMENTS,
            &[number, text, descriptor],
        );
    }
    exception
}

/// Returns a new exception of the class for `errno`, written as
/// `[Errno 2] No such file or directory: 'name'` with the `repr` of `filename`, or without
/// it when `filename` is null, and holding the arguments `(errno, strerror)`.
///
/// # Safety
///
/// `filename` is null or points to a live `str`.
pub(crate) unsafe fn os_error(errno: i32, filename: *const StrObject) -> *mut ExceptionObject {
    let strerror = text_of(errno);
    let mut message = MessageBuffer::new();
    message.push_str("[Errno ");
    message.push_number(i64::from(errno));
    message.push_str("] ");
    // SAFETY: the text is a live `str`, and the caller passes a live file name or null.
    unsafe {
        message.push_text(strerror);
        if !filename.is_null() {
            message.push_str(": ");
            message.push_repr(filename);
        }
    }
    let exception = crate::exception::raise_class(class_of(errno), message.text());
    let number = crate::word::inline(i64::from(errno))
        .unwrap_or(1)
        .cast_unsigned();
    // SAFETY: the exception is new and has no arguments; it takes the text's reference, and
    // one of its own to the file name.
    unsafe {
        let text = strerror.expose_provenance() as u64;
        if filename.is_null() {
            crate::exception::set_arguments(exception, &ERROR_ARGUMENTS, &[number, text]);
        } else {
            crate::refcount::typy_rt_increment(filename.cast_mut().cast());
            let name = filename.expose_provenance() as u64;
            crate::exception::set_arguments(
                exception,
                &NAMED_ERROR_ARGUMENTS,
                &[number, text, name],
            );
        }
    }
    exception
}

/// Returns a new `OSError` of `class` built with the arguments `items` of the layout
/// `arguments_type`, two to five of any type, read as `errno`, `strerror`, `filename`,
/// `winerror` and `filename2`. `OSError` itself becomes the subclass for `errno` when it is
/// an `int` or a `bool`, as CPython's `OSError.__new__` looks it up.
///
/// # Safety
///
/// `class` is `OSError` or a class derived from it, and `items` a live tuple of
/// `arguments_type`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_os_error_new(
    class: *const TypeObject,
    arguments_type: *const TupleType,
    items: *const u8,
) -> *mut ExceptionObject {
    let address = items.expose_provenance() as u64;
    // SAFETY: the caller's guarantee.
    unsafe {
        let class = if core::ptr::eq(class, &raw const OS_ERROR) {
            errno_number(arguments_type, address)
                .and_then(|number| i32::try_from(number).ok())
                .map_or(&OS_ERROR, class_of)
        } else {
            &*class
        };
        crate::exception::typy_rt_exception_new(class, arguments_type, items)
    }
}

/// Returns the first item of an `OSError`'s arguments as a number when it is an `int` that
/// fits a word, or a `bool`, which CPython counts as an `int`.
///
/// # Safety
///
/// `address` is a live tuple of `layout`, which has at least one item.
unsafe fn errno_number(layout: *const TupleType, address: u64) -> Option<i64> {
    // SAFETY: the caller's guarantee.
    let (ty, value) = unsafe {
        crate::tuple::field_type(layout, 0).held_value(crate::tuple::field(layout, address, 0))?
    };
    let word = value.cast_signed();
    match ty.kind() {
        ItemKind::Int if crate::word::is_inline(word) => Some(crate::word::inline_value(word)),
        ItemKind::Bool => Some(i64::from(value & 1 != 0)),
        _ => None,
    }
}

/// Returns `str` of an `OSError`: the runtime's text for one it raised; for one built with
/// two to five arguments, `[Errno 2] text`, with `: 'name'` when the file name is not
/// `None` and ` -> 'other'` when the second one is not either, `errno` and `strerror` written
/// with `str` and the file names with `repr`, as CPython's `OSError_str` writes them;
/// otherwise what `BaseException` writes.
///
/// # Safety
///
/// `object` points to a live `OSError` or an exception of a class derived from it.
#[unsafe(no_mangle)]
#[allow(
    clippy::cast_ptr_alignment,
    reason = "every object comes from malloc, a frame block or a static, aligned for every type, and starts with its header"
)]
pub unsafe extern "C" fn typy_rt_os_error_str(
    object: *mut ObjectHeader,
) -> Raising<*mut StrObject> {
    // SAFETY: the caller's guarantee.
    let exception = unsafe { &*object.cast::<ExceptionObject>() };
    let layout = exception.arguments_type;
    let length = if exception.message.is_null() && !exception.arguments.is_null() {
        // SAFETY: an exception with arguments has a live descriptor.
        usize::try_from(unsafe { (*layout).length }).unwrap_or(0)
    } else {
        0
    };
    if !(2..=5).contains(&length) {
        // SAFETY: the caller's guarantee.
        return unsafe { crate::exception::typy_rt_exception_str(object) };
    }
    let address = exception.arguments.expose_provenance() as u64;
    // SAFETY: the arguments are live and laid out as their descriptor says.
    let present = |index: usize| unsafe {
        index < length
            && crate::tuple::field_type(layout, index)
                .held_value(crate::tuple::field(layout, address, index))
                .is_some_and(|(ty, _)| ty.kind() != ItemKind::NoneValue)
    };
    let filename = present(2);
    let filename2 = filename && present(4);
    // SAFETY: the arguments are live and laid out as their descriptor says.
    let written = |index: usize, repr: bool| unsafe {
        let ty = crate::tuple::field_type(layout, index);
        let item = crate::tuple::field(layout, address, index);
        if repr { ty.repr(item) } else { ty.str(item) }
    };
    let mut text = Joined::new("[Errno ");
    let built = (|| {
        text.add(written(0, false))?;
        text.add_str("] ");
        text.add(written(1, false))?;
        if filename {
            text.add_str(": ");
            text.add(written(2, true))?;
        }
        if filename2 {
            text.add_str(" -> ");
            text.add(written(4, true))?;
        }
        Ok(())
    })();
    match built {
        Ok(()) => Raising::ok(text.0),
        Err(exception) => {
            // SAFETY: the text is live and owned here.
            unsafe { crate::refcount::typy_rt_decrement(text.0.cast()) };
            Raising::error(core::ptr::null_mut(), exception)
        }
    }
}

/// A `str` built by joining pieces, owned until it is given away.
struct Joined(*mut StrObject);

impl Joined {
    /// Returns a text that starts with `start`.
    fn new(start: &str) -> Self {
        Self(crate::string::from_str(start))
    }

    /// Appends `piece`, which is released.
    fn add_owned(&mut self, piece: *mut StrObject) {
        // SAFETY: both strings are live and owned here, and each is released once.
        unsafe {
            let joined = crate::string::typy_rt_str_concat(self.0, piece);
            crate::refcount::typy_rt_decrement(self.0.cast());
            crate::refcount::typy_rt_decrement(piece.cast());
            self.0 = joined;
        }
    }

    /// Appends `piece`.
    fn add_str(&mut self, piece: &str) {
        self.add_owned(crate::string::from_str(piece));
    }

    /// Appends the text a writer gave, or returns the exception it raised.
    fn add(&mut self, written: Raising<*mut StrObject>) -> Result<(), *mut ExceptionObject> {
        if !written.exception.is_null() {
            return Err(written.exception);
        }
        self.add_owned(written.value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{typy_rt_os_error_new, typy_rt_os_error_str};
    use crate::exception::classes::{FILE_NOT_FOUND_ERROR, OS_ERROR};
    use crate::list::ItemKind;
    use crate::refcount::typy_rt_decrement;
    use crate::string::{StrObject, code_point, from_str, length};
    use crate::tuple::{Field, TupleType};
    use crate::type_object::TypeObject;
    use crate::word::inline;

    /// Returns the text of a string, and releases it.
    fn taken(string: *mut StrObject) -> String {
        // SAFETY: the string is live and owned here.
        unsafe {
            let text = (0..length(string))
                .map(|index| char::from_u32(code_point(string, index)).unwrap())
                .collect();
            typy_rt_decrement(string.cast());
            text
        }
    }

    /// Returns a layout of these kinds, eight bytes apart.
    fn layout(kinds: &[ItemKind]) -> &'static TupleType {
        let fields: Vec<Field> = kinds
            .iter()
            .zip(0..)
            .map(|(kind, index)| Field::plain(*kind, index * 8))
            .collect();
        let fields = fields.leak();
        Box::leak(Box::new(TupleType {
            size: 8 * fields.len() as u64,
            length: fields.len() as u64,
            fields: fields.as_ptr(),
        }))
    }

    /// Returns the class name and `str` of an `OSError` of `class` built with these items.
    fn built(class: &TypeObject, kinds: &[ItemKind], items: &[u64]) -> (&'static str, String) {
        // SAFETY: the items are live values of the layout, which the exception copies.
        unsafe {
            let error = typy_rt_os_error_new(class, layout(kinds), items.as_ptr().cast());
            let written = typy_rt_os_error_str(error.cast());
            assert!(written.exception.is_null());
            let found = ((*error).class().name(), taken(written.value));
            typy_rt_decrement(error.cast());
            found
        }
    }

    #[test]
    fn os_error_picks_its_subclass_only_for_a_small_int_or_a_bool() {
        let gone = from_str("gone");
        let word = |object: *mut StrObject| object.expose_provenance() as u64;
        let small = |value: i64| inline(value).unwrap().cast_unsigned();
        let big = crate::int::value::Integer {
            negative: false,
            magnitude: crate::int::natural::shift_left(&[1], 70),
        }
        .into_word();
        let (int, text_kind) = (ItemKind::Int, ItemKind::Object);
        let cases = [
            (
                &OS_ERROR,
                int,
                small(2),
                "FileNotFoundError",
                "[Errno 2] gone",
            ),
            (
                &OS_ERROR,
                int,
                big.cast_unsigned(),
                "OSError",
                "[Errno 1180591620717411303424] gone",
            ),
            (
                &OS_ERROR,
                ItemKind::Bool,
                1,
                "PermissionError",
                "[Errno True] gone",
            ),
            (
                &OS_ERROR,
                text_kind,
                word(gone),
                "OSError",
                "[Errno gone] gone",
            ),
            (
                &FILE_NOT_FOUND_ERROR,
                int,
                small(13),
                "FileNotFoundError",
                "[Errno 13] gone",
            ),
        ];
        for (class, kind, errno, name, text) in cases {
            assert_eq!(
                built(class, &[kind, text_kind], &[errno, word(gone)]),
                (name, text.to_owned()),
                "{text}"
            );
        }
        // SAFETY: the test holds the only references.
        unsafe {
            crate::refcount::typy_rt_int_decrement(big);
            typy_rt_decrement(gone.cast());
        }
    }

    #[test]
    fn the_file_names_are_written_with_repr_unless_they_are_none() {
        let (gone, name, other) = (from_str("gone"), from_str("a"), from_str("b"));
        let word = |object: *mut StrObject| object.expose_provenance() as u64;
        let two = inline(2).unwrap().cast_unsigned();
        let (int, text_kind) = (ItemKind::Int, ItemKind::Object);
        let optional = ItemKind::OptionalObject;
        let five = [int, text_kind, optional, ItemKind::NoneValue, optional];
        let cases: [(&[ItemKind], [u64; 5], &str); 4] = [
            (
                &five,
                [two, word(gone), word(name), 0, word(other)],
                "[Errno 2] gone: 'a' -> 'b'",
            ),
            (
                &five,
                [two, word(gone), 0, 0, word(other)],
                "[Errno 2] gone",
            ),
            (
                &five,
                [two, word(gone), word(name), 0, 0],
                "[Errno 2] gone: 'a'",
            ),
            (
                &[int, text_kind, int],
                [two, word(gone), two, 0, 0],
                "[Errno 2] gone: 2",
            ),
        ];
        for (kinds, items, text) in cases {
            assert_eq!(
                built(&OS_ERROR, kinds, &items[..kinds.len()]).1,
                text,
                "{text}"
            );
        }
        // SAFETY: the test holds the only references.
        unsafe {
            for object in [gone, name, other] {
                typy_rt_decrement(object.cast());
            }
        }
    }

    #[test]
    fn an_os_error_of_one_argument_or_of_six_is_written_as_base_exception_writes_it() {
        let plain = crate::exception::raise_class(&OS_ERROR, from_str("plain"));
        let one = from_str("one");
        let word = one.expose_provenance() as u64;
        let six = [ItemKind::Object; 6];
        // SAFETY: the exceptions and the string are live and owned here.
        unsafe {
            assert_eq!(taken(typy_rt_os_error_str(plain.cast()).value), "plain");
            typy_rt_decrement(plain.cast());
            let error = crate::exception::typy_rt_exception_new(
                &raw const OS_ERROR,
                layout(&six),
                [word; 6].as_ptr().cast(),
            );
            assert_eq!(
                taken(typy_rt_os_error_str(error.cast()).value),
                "('one', 'one', 'one', 'one', 'one', 'one')"
            );
            typy_rt_decrement(error.cast());
            typy_rt_decrement(one.cast());
        }
    }

    #[test]
    fn an_error_about_a_descriptor_holds_it_as_the_file_name() {
        let error = super::os_error_of_descriptor(21, 3);
        // SAFETY: the exception is live and owned here, with three arguments.
        unsafe {
            assert_eq!((*error).class().name(), "IsADirectoryError");
            let layout = (*error).arguments_type;
            let address = (*error).arguments.expose_provenance() as u64;
            assert_eq!((*layout).length, 3);
            assert_eq!(crate::tuple::field_type(layout, 2).kind(), ItemKind::Int);
            assert_eq!(
                crate::tuple::field(layout, address, 2),
                inline(3).unwrap().cast_unsigned()
            );
            assert_eq!(
                taken(typy_rt_os_error_str(error.cast()).value),
                "[Errno 21] Is a directory: 3"
            );
            typy_rt_decrement(error.cast());
        }
    }

    #[test]
    fn an_errno_whose_str_raises_raises_from_str() {
        let gone = from_str("gone");
        // SAFETY: both operands are `int` words; `10 ** 5000` has more digits than `str`
        // writes.
        let large =
            unsafe { crate::int::typy_rt_int_power(inline(10).unwrap(), inline(5000).unwrap()) }
                .value;
        let items = [large.cast_unsigned(), gone.expose_provenance() as u64];
        // SAFETY: the items are live values of the layout, which the exception copies.
        unsafe {
            let error = typy_rt_os_error_new(
                &raw const OS_ERROR,
                layout(&[ItemKind::Int, ItemKind::Object]),
                items.as_ptr().cast(),
            );
            let written = typy_rt_os_error_str(error.cast());
            assert!(!written.exception.is_null());
            typy_rt_decrement(written.exception.cast());
            typy_rt_decrement(error.cast());
            crate::refcount::typy_rt_int_decrement(large);
            typy_rt_decrement(gone.cast());
        }
    }
}
