//! `sys.version_info`, a value of its own (LIB-16): one immortal object of the type
//! `sys.version_info`, written as CPython 3.15.0 writes it, whose behaviour TyPy follows
//! (ADR-0029). Its items are read when the program is compiled; the object is what a program
//! stores, passes and writes.

use crate::Raising;
use crate::header::{IMMORTAL, ObjectHeader, VERSION_INFO_TYPE_ID};
use crate::string::StrObject;
use crate::type_object::{OBJECT, TypeObject};

/// The text of `sys.version_info`.
const TEXT: &str = "sys.version_info(major=3, minor=15, micro=0, releaselevel='final', serial=0)";

/// The type `sys.version_info`.
#[unsafe(export_name = "typy_rt_type_version_info")]
pub(crate) static VERSION_INFO_TYPE: TypeObject = TypeObject {
    hash: Some(crate::instance::typy_rt_identity_hash),
    repr: Some(version_info_repr),
    module: "sys".as_ptr(),
    module_length: 3,
    ..TypeObject::new("version_info", &raw const OBJECT)
};

/// The one `sys.version_info` object.
static VERSION_INFO: ObjectHeader = ObjectHeader {
    reference_count: IMMORTAL,
    type_id: VERSION_INFO_TYPE_ID,
};

/// Returns `sys.version_info`, which the runtime keeps for the whole run.
#[unsafe(no_mangle)]
pub extern "C" fn typy_rt_sys_version_info() -> *const ObjectHeader {
    &raw const VERSION_INFO
}

/// Returns `repr` of `sys.version_info`.
unsafe extern "C" fn version_info_repr(_object: *mut ObjectHeader) -> Raising<*mut StrObject> {
    Raising::ok(crate::string::from_str(TEXT))
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_info_reads_as_cpython_3_15_0_writes_it() {
        // SAFETY: the object is the runtime's static.
        unsafe {
            let object = super::typy_rt_sys_version_info().cast_mut();
            let text = super::version_info_repr(object).value;
            let written: String = (0..crate::string::length(text))
                .map(|index| char::from_u32(crate::string::code_point(text, index)).unwrap())
                .collect();
            crate::refcount::typy_rt_decrement(text.cast());
            assert_eq!(
                written,
                "sys.version_info(major=3, minor=15, micro=0, releaselevel='final', serial=0)"
            );
            let ty = crate::type_object::typy_rt_type_text(&raw const super::VERSION_INFO_TYPE);
            let written: String = (0..crate::string::length(ty))
                .map(|index| char::from_u32(crate::string::code_point(ty, index)).unwrap())
                .collect();
            crate::refcount::typy_rt_decrement(ty.cast());
            assert_eq!(written, "<class 'sys.version_info'>");
        }
    }
}
