//! The names of the `sys` module (LIB-04, LIB-13, MOD-09, ADR-0018): the ones TyPy provides, the switches of lazy imports it refuses, and the rest of the
//! names CPython 3.12 to 3.15 define, which TyPy does not provide.

/// A value of `sys`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysValue {
    /// `sys.argv`, a `list[str]`.
    Argv,
    /// `sys.stdout`.
    Stdout,
    /// `sys.stderr`.
    Stderr,
    /// `sys.stdin`, a text file read from standard input (LIB-13).
    Stdin,
}

/// A constant of `sys`, the same for every program TyPy builds (LIB-13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SysConstant {
    /// An `int`, by its decimal digits.
    Int(&'static str),
    /// A `str`.
    Str(&'static str),
}

/// What a name of `sys` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SysName {
    /// A value TyPy provides.
    Value(SysValue),
    /// `sys.exit`.
    Exit,
    /// A constant: `maxsize`, `maxunicode`, `byteorder` or `platform`.
    Constant(SysConstant),
    /// `sys.getrecursionlimit`.
    GetRecursionLimit,
    /// `sys.setrecursionlimit`.
    SetRecursionLimit,
    /// A switch of lazy imports, which changes how imports behave from outside the module
    /// that imports (MOD-09).
    LazySwitch,
    /// `sys.version_info`, which TyPy folds where a program compares or reads it, and plans
    /// as a value (LIB-13, LIB-16).
    VersionInfo,
    /// A name of CPython's `sys` that TyPy does not provide.
    Unprovided,
}

/// The switches and the state of lazy imports, in CPython 3.15.
const LAZY_SWITCHES: [&str; 5] = [
    "get_lazy_imports",
    "get_lazy_imports_filter",
    "lazy_modules",
    "set_lazy_imports",
    "set_lazy_imports_filter",
];

/// The other public names of CPython 3.12 to 3.15's `sys`, in name order.
const OTHERS: [&str; 66] = [
    "abi_info",
    "abiflags",
    "activate_stack_trampoline",
    "addaudithook",
    "api_version",
    "audit",
    "base_exec_prefix",
    "base_prefix",
    "breakpointhook",
    "builtin_module_names",
    "call_tracing",
    "copyright",
    "deactivate_stack_trampoline",
    "displayhook",
    "dont_write_bytecode",
    "exc_info",
    "excepthook",
    "exception",
    "exec_prefix",
    "executable",
    "flags",
    "float_info",
    "float_repr_style",
    "get_asyncgen_hooks",
    "get_coroutine_origin_tracking_depth",
    "get_int_max_str_digits",
    "getallocatedblocks",
    "getdefaultencoding",
    "getdlopenflags",
    "getfilesystemencodeerrors",
    "getfilesystemencoding",
    "getprofile",
    "getrefcount",
    "getsizeof",
    "getswitchinterval",
    "gettrace",
    "getunicodeinternedsize",
    "hash_info",
    "implementation",
    "int_info",
    "intern",
    "is_finalizing",
    "is_remote_debug_enabled",
    "is_stack_trampoline_active",
    "meta_path",
    "modules",
    "monitoring",
    "orig_argv",
    "path",
    "path_hooks",
    "path_importer_cache",
    "platlibdir",
    "prefix",
    "pycache_prefix",
    "remote_exec",
    "set_asyncgen_hooks",
    "set_coroutine_origin_tracking_depth",
    "set_int_max_str_digits",
    "setdlopenflags",
    "setprofile",
    "setswitchinterval",
    "settrace",
    "stdlib_module_names",
    "thread_info",
    "unraisablehook",
    "warnoptions",
];

/// Returns what `name` is in `sys`, if it is a name of the module in some CPython version.
pub(crate) fn sys_name(name: &str) -> Option<SysName> {
    Some(match name {
        "argv" => SysName::Value(SysValue::Argv),
        "stdout" => SysName::Value(SysValue::Stdout),
        "stderr" => SysName::Value(SysValue::Stderr),
        "stdin" => SysName::Value(SysValue::Stdin),
        "exit" => SysName::Exit,
        // TyPy 0.4 builds for Linux x86_64 only (SRS 2.3); `platform` follows the target
        // once macOS is one.
        "maxsize" => SysName::Constant(SysConstant::Int("9223372036854775807")),
        "maxunicode" => SysName::Constant(SysConstant::Int("1114111")),
        "byteorder" => SysName::Constant(SysConstant::Str("little")),
        "platform" => SysName::Constant(SysConstant::Str("linux")),
        // The version of CPython whose behaviour TyPy follows (ADR-0029).
        "version" => SysName::Constant(SysConstant::Str(concat!(
            "3.15.0 (TyPy ",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))),
        "hexversion" => SysName::Constant(SysConstant::Int("51314928")),
        "version_info" => SysName::VersionInfo,
        "getrecursionlimit" => SysName::GetRecursionLimit,
        "setrecursionlimit" => SysName::SetRecursionLimit,
        _ if LAZY_SWITCHES.contains(&name) => SysName::LazySwitch,
        _ if OTHERS.binary_search(&name).is_ok() => SysName::Unprovided,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
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
        assert_eq!(sys_name("version_info"), Some(SysName::VersionInfo));
        assert_eq!(sys_name("settrace"), Some(SysName::Unprovided));
        assert_eq!(sys_name("argvs"), None);
    }
}
