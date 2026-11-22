//! Tests of `sys`.

use super::{OTHERS, SysConstant, SysName, SysValue, sys_name};

#[test]
fn each_kind_of_name_is_found() {
    assert!(OTHERS.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(sys_name("argv"), Some(SysName::Value(SysValue::Argv)));
    assert_eq!(sys_name("stdout"), Some(SysName::Value(SysValue::Stdout)));
    assert_eq!(sys_name("stderr"), Some(SysName::Value(SysValue::Stderr)));
    assert_eq!(sys_name("exit"), Some(SysName::Exit));
    assert_eq!(sys_name("set_lazy_imports"), Some(SysName::LazySwitch));
    assert_eq!(sys_name("stdin"), Some(SysName::Value(SysValue::Stdin)));
    assert_eq!(
        sys_name("byteorder"),
        Some(SysName::Constant(SysConstant::Str("little")))
    );
    assert_eq!(
        sys_name("setrecursionlimit"),
        Some(SysName::SetRecursionLimit)
    );
    assert_eq!(
        sys_name("version_info"),
        Some(SysName::Value(SysValue::VersionInfo))
    );
    assert_eq!(sys_name("settrace"), Some(SysName::Unprovided));
    assert_eq!(sys_name("argvs"), None);
}
