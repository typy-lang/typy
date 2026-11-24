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
mod tests;
