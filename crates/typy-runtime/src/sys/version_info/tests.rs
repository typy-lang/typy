//! Tests of `sys.version_info`.

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
