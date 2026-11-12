//! The `sys` module (LIB-04): the program's arguments, `input`, and the exit status an
//! uncaught `SystemExit` gives.

use crate::Raising;
use crate::cell::ProgramCell;
use crate::exception::{ExceptionKind, ExceptionObject, raise};
use crate::list::ListObject;
use crate::string::{StrBuilder, StrObject};

/// The program's arguments as the C library gave them to `main`, and the list `sys.argv`
/// made of them once read.
struct Arguments {
    count: usize,
    values: *const *const u8,
    list: *mut ListObject,
}

static ARGUMENTS: ProgramCell<Arguments> = ProgramCell::new(Arguments {
    count: 0,
    values: core::ptr::null(),
    list: core::ptr::null_mut(),
});

/// Keeps the arguments `main` receives, for `sys.argv`.
pub(crate) fn keep_arguments(count: i32, values: *const *const u8) {
    ARGUMENTS.with(|arguments| {
        arguments.count = usize::try_from(count).unwrap_or(0);
        arguments.values = values;
    });
}

/// Returns `bytes` decoded as UTF-8, each byte that is not part of a character as U+DC80 to
/// U+DCFF, as CPython decodes its arguments and standard input in UTF-8 mode.
pub(crate) fn decode(bytes: &[u8]) -> *mut StrObject {
    let mut text = StrBuilder::with_capacity(bytes.len());
    let mut at = 0;
    let byte = |index: usize| bytes.get(index).map(|byte| u32::from(*byte));
    while at < bytes.len() {
        if let Ok((count, point)) = crate::codec::utf8_sequence(byte, at) {
            text.push(point);
            at += count;
        } else {
            text.push(0xdc00 + u32::from(bytes[at]));
            at += 1;
        }
    }
    text.finish()
}

/// Returns `sys.argv`, a list of the program's arguments made the first time it is read and
/// kept for the whole run, so a change to it shows at the next read.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_sys_argv() -> *mut ListObject {
    ARGUMENTS.with(|arguments| {
        if arguments.list.is_null() {
            let list = crate::list::new_list(crate::list::ItemKind::Object, arguments.count);
            for index in 0..arguments.count {
                // SAFETY: the C library passes `count` arguments, each a string ended by a
                // zero byte.
                let bytes = unsafe {
                    let start = *arguments.values.add(index);
                    let mut end = 0;
                    while *start.add(end) != 0 {
                        end += 1;
                    }
                    core::slice::from_raw_parts(start, end)
                };
                // SAFETY: the list is live and takes the new string's reference.
                unsafe { crate::list::push_retained(list, decode(bytes) as u64) };
            }
            arguments.list = list;
        }
        arguments.list
    })
}

/// What `input` has read from standard input but not given out yet.
struct Input {
    bytes: [u8; INPUT_BLOCK],
    start: usize,
    end: usize,
}

/// The bytes `input` asks standard input for at a time.
const INPUT_BLOCK: usize = 4096;

static INPUT: ProgramCell<Input> = ProgramCell::new(Input {
    bytes: [0; INPUT_BLOCK],
    start: 0,
    end: 0,
});

/// Returns `input(prompt)`: the prompt written to standard output, which is then flushed,
/// and the next line of standard input without its `\n`, or `EOFError` at its end.
///
/// # Safety
///
/// `prompt` is null or points to a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_input(prompt: *const StrObject) -> Raising<*mut StrObject> {
    if !prompt.is_null() {
        // SAFETY: the caller's guarantee.
        unsafe { crate::io::typy_rt_write_stdout(prompt) };
    }
    crate::io::flush_stdout();
    INPUT.with(|input| read_line(input, |bytes| crate::libc::read(crate::libc::STDIN, bytes)))
}

/// Returns the next line of `input`, which `read` refills when it is empty, decoded and
/// without its `\n`; the last line may end without one. Raises `EOFError` when `read` gives
/// nothing before a line starts.
fn read_line(
    input: &mut Input,
    mut read: impl FnMut(&mut [u8]) -> usize,
) -> Raising<*mut StrObject> {
    let mut line = Line::new();
    let mut read_any = false;
    loop {
        if input.start == input.end {
            input.start = 0;
            input.end = read(&mut input.bytes);
            if input.end == 0 {
                break;
            }
        }
        read_any = true;
        let rest = &input.bytes[input.start..input.end];
        if let Some(at) = rest.iter().position(|byte| *byte == b'\n') {
            line.extend(&rest[..at]);
            input.start += at + 1;
            break;
        }
        line.extend(rest);
        input.start = input.end;
    }
    if !read_any {
        line.discard();
        return Raising::error(
            core::ptr::null_mut(),
            raise(ExceptionKind::EOFError, "EOF when reading a line"),
        );
    }
    let text = decode(line.bytes());
    line.discard();
    Raising::ok(text)
}

/// The bytes of a line `input` reads, in memory of their own.
struct Line {
    bytes: *mut u8,
    length: usize,
    capacity: usize,
}

impl Line {
    fn new() -> Self {
        Self {
            bytes: crate::allocator::allocate(64),
            length: 0,
            capacity: 64,
        }
    }

    fn extend(&mut self, bytes: &[u8]) {
        let needed = self.length + bytes.len();
        if needed > self.capacity {
            self.capacity = needed.next_power_of_two();
            self.bytes = crate::allocator::reallocate(self.bytes, self.capacity);
        }
        // SAFETY: the allocation holds `capacity` bytes, at least `length` more than the
        // bytes copied, and the source is a different allocation.
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.bytes.add(self.length),
                bytes.len(),
            );
        }
        self.length = needed;
    }

    fn bytes(&self) -> &[u8] {
        // SAFETY: the first `length` bytes are written.
        unsafe { core::slice::from_raw_parts(self.bytes, self.length) }
    }

    fn discard(self) {
        crate::allocator::free(self.bytes);
    }
}

/// Returns the exit status an uncaught exception gives, and whether it is reported: a
/// `SystemExit`, or an instance of a subclass, without an argument or with `None` ends with
/// 0, with an `int` with that number modulo 256 (255 for one outside `i128`, as CPython 3.15
/// gives), and with anything else, which is written to standard error, with 1; any other
/// exception is reported and ends with 1.
///
/// # Safety
///
/// `exception` points to a live exception.
pub(crate) unsafe fn exit_status(exception: *mut ExceptionObject) -> (i32, bool) {
    // SAFETY: the caller's guarantee.
    let object = unsafe { &*exception };
    if !is_system_exit(object.class()) {
        return (1, true);
    }
    match crate::exception::exit_code(object) {
        crate::exception::ExitCode::None => (0, false),
        crate::exception::ExitCode::Int(word) => {
            let low = if crate::word::is_inline(word) {
                i128::from(crate::word::inline_value(word))
            } else {
                // SAFETY: the word is the exception's own, live while it is.
                unsafe { crate::int::value::Integer::from_word(word) }
                    .to_i128()
                    .unwrap_or(-1)
            };
            #[allow(
                clippy::cast_possible_truncation,
                reason = "the status is the low byte, as the operating system keeps it"
            )]
            let status = (low & 0xff) as i32;
            (status, false)
        }
        crate::exception::ExitCode::Other => {
            if let Some(str) = object.class().str {
                // SAFETY: the exception is live; its class's `str` gives the `str` of its
                // argument.
                unsafe {
                    let text = str(exception.cast());
                    if text.exception.is_null() {
                        crate::string::write_utf8(text.value, crate::io::write_stderr);
                        crate::io::write_stderr(b"\n");
                        crate::refcount::typy_rt_decrement(text.value.cast());
                    } else {
                        crate::refcount::typy_rt_decrement(text.exception.cast());
                    }
                }
            }
            (1, false)
        }
    }
}

/// Returns whether `class` is `SystemExit` or derives from it. `SystemExit` is found by its
/// name among the built-in classes, whose module is empty, and not by its address, so `main`
/// does not link the exception classes and their text into a program that never raises one.
fn is_system_exit(class: &crate::type_object::TypeObject) -> bool {
    let mut current: *const crate::type_object::TypeObject = class;
    while !current.is_null() {
        // SAFETY: a base is null or a live, immortal type object.
        let ancestor = unsafe { &*current };
        if ancestor.name() == "SystemExit" && ancestor.module().is_empty() {
            return true;
        }
        current = ancestor.base;
    }
    false
}

#[cfg(test)]
mod tests;
