//! The errors of reading module variables, raised with CPython's messages (SMT-08, ADR-0015).

use crate::exception::{ExceptionKind, ExceptionObject};
use crate::message::MessageBuffer;
use crate::string::StrObject;

/// Returns `NameError: name 'x' is not defined`, raised by a function that reads the module
/// variable `name` while it is not bound.
///
/// # Safety
///
/// `name` points to a live `str` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn typy_rt_name_error(name: *const StrObject) -> *mut ExceptionObject {
    let mut message = MessageBuffer::new();
    message.push_str("name '");
    // SAFETY: the caller's guarantee.
    unsafe { message.push_text(name) };
    message.push_str("' is not defined");
    message.raise(ExceptionKind::NameError)
}

#[cfg(test)]
mod tests {
    use super::typy_rt_name_error;
    use crate::string::from_str;

    #[test]
    fn an_unbound_module_variable_raises_name_error_with_cpythons_message() {
        let name = from_str("count");
        // SAFETY: the name is live, and the exception is owned here.
        unsafe {
            let exception = typy_rt_name_error(name);
            assert_eq!((*exception).kind(), crate::ExceptionKind::NameError);
            let text = crate::exception::classes::BASE_EXCEPTION.str.unwrap()(exception.cast());
            let shown: String = (0..crate::string::length(text.value))
                .map(|index| char::from_u32(crate::string::code_point(text.value, index)).unwrap())
                .collect();
            assert_eq!(shown, "name 'count' is not defined");
            crate::refcount::typy_rt_decrement(text.value.cast());
            crate::refcount::typy_rt_decrement(exception.cast());
            crate::refcount::typy_rt_decrement(name.cast());
        }
    }
}
