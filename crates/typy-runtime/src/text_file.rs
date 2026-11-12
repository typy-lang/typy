//! Text files, `_io.TextIOWrapper` (LIB-04): standard output and standard error, written as
//! UTF-8, standard output through its buffer.

use crate::Raising;
use crate::exception::ExceptionObject;
use crate::header::{IMMORTAL, ObjectHeader, TEXT_FILE_TYPE_ID};
use crate::message::MessageBuffer;
use crate::string::{StrObject, length, write_utf8};
use crate::type_object::{OBJECT, TypeObject};

/// A text file: the descriptor it writes to and the name `repr` shows.
#[repr(C)]
#[derive(Debug)]
pub struct TextFile {
    header: ObjectHeader,
    descriptor: i32,
    name: &'static str,
}

/// `sys.stdout`.
static STDOUT: TextFile = TextFile {
    header: ObjectHeader {
        reference_count: IMMORTAL,
        type_id: TEXT_FILE_TYPE_ID,
    },
    descriptor: crate::libc::STDOUT,
    name: "<stdout>",
};

/// `sys.stderr`.
static STDERR: TextFile = TextFile {
    header: ObjectHeader {
        reference_count: IMMORTAL,
        type_id: TEXT_FILE_TYPE_ID,
    },
    descriptor: crate::libc::STDERR,
    name: "<stderr>",
};

/// The type `_io.TextIOWrapper`.
#[unsafe(export_name = "typy_rt_type_text_file")]
pub(crate) static TEXT_FILE: TypeObject = TypeObject {
    hash: Some(crate::instance::typy_rt_identity_hash),
    repr: Some(text_file_repr),
    module: "_io".as_ptr(),
    module_length: 3,
    ..TypeObject::new("TextIOWrapper", &raw const OBJECT)
};

/// Returns `repr` of a text file, as CPython writes it:
/// `<_io.TextIOWrapper name='<stdout>' mode='w' encoding='utf-8'>`.
#[allow(
    clippy::cast_ptr_alignment,
    reason = "a text file is a static the runtime aligns for its fields"
)]
unsafe extern "C" fn text_file_repr(object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    // SAFETY: the type of the object says it is a text file.
    let file = unsafe { &*object.cast::<TextFile>() };
    let mut text = MessageBuffer::new();
    text.push_str("<_io.TextIOWrapper name='");
    text.push_str(file.name);
    text.push_str("' mode='w' encoding='utf-8'>");
    Raising::ok(text.text())
}

/// Returns `sys.stdout`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_sys_stdout() -> *const TextFile {
    &raw const STDOUT
}

/// Returns `sys.stderr`.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_sys_stderr() -> *const TextFile {
    &raw const STDERR
}

/// Writes `text` to `file`, or to standard output when `file` is null, as `print(file=None)`
/// does, and returns the number of code points written, an `int` word.
///
/// # Safety
///
/// `file` is null or points to a live text file, and `text` to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_write(
    file: *const TextFile,
    text: *const StrObject,
) -> Raising<i64> {
    let descriptor = if file.is_null() {
        crate::libc::STDOUT
    } else {
        // SAFETY: the caller's guarantee.
        unsafe { (*file).descriptor }
    };
    // SAFETY: the caller's guarantee.
    unsafe {
        if descriptor == crate::libc::STDOUT {
            crate::io::typy_rt_write_stdout(text);
        } else {
            write_utf8(text, crate::io::write_stderr);
        }
    }
    // SAFETY: the caller passes a live string.
    let count = i64::try_from(unsafe { length(text) }).unwrap_or(i64::MAX);
    Raising::ok(crate::word::inline(count).unwrap_or(1))
}

/// Writes out what standard output holds when `file` is standard output or null; standard
/// error holds nothing.
///
/// # Safety
///
/// `file` is null or points to a live text file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_text_flush(file: *const TextFile) -> *mut ExceptionObject {
    // SAFETY: the caller's guarantee.
    if file.is_null() || unsafe { (*file).descriptor } == crate::libc::STDOUT {
        crate::io::flush_stdout();
    }
    core::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::{
        TEXT_FILE, typy_rt_sys_stderr, typy_rt_sys_stdout, typy_rt_text_flush, typy_rt_text_write,
    };
    use crate::refcount::typy_rt_decrement;
    use crate::string::{code_point, from_str, length};

    fn repr_of(file: *const super::TextFile) -> String {
        let repr = TEXT_FILE.repr.unwrap();
        // SAFETY: the file is a live text file, and the text is owned here.
        unsafe {
            let text = repr(file.cast_mut().cast()).value;
            let found = (0..length(text))
                .map(|index| char::from_u32(code_point(text, index)).unwrap())
                .collect();
            typy_rt_decrement(text.cast());
            found
        }
    }

    #[test]
    fn the_standard_files_show_as_cpython_shows_them() {
        assert_eq!(
            repr_of(typy_rt_sys_stdout()),
            "<_io.TextIOWrapper name='<stdout>' mode='w' encoding='utf-8'>"
        );
        assert_eq!(
            repr_of(typy_rt_sys_stderr()),
            "<_io.TextIOWrapper name='<stderr>' mode='w' encoding='utf-8'>"
        );
        assert_eq!(TEXT_FILE.name(), "TextIOWrapper");
    }

    #[test]
    fn write_returns_the_number_of_code_points_for_each_file() {
        let text = from_str("caf\u{e9}\n");
        let expected = crate::word::inline(5).unwrap();
        // SAFETY: the files and the text are live; the text is owned here.
        unsafe {
            for file in [
                typy_rt_sys_stdout(),
                typy_rt_sys_stderr(),
                core::ptr::null(),
            ] {
                let written = typy_rt_text_write(file, text);
                assert!(written.exception.is_null());
                assert_eq!(written.value, expected);
                assert!(typy_rt_text_flush(file).is_null());
            }
            typy_rt_decrement(text.cast());
        }
    }
}
