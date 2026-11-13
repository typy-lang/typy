//! The names of the `sys` module (LIB-04, MOD-09, ADR-0018): the four TyPy 0.3 provides,
//! the ones planned for 0.4, the switches of lazy imports it refuses, and the rest of the
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
}

/// What a name of `sys` is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SysName {
    /// A value TyPy provides.
    Value(SysValue),
    /// `sys.exit`.
    Exit,
    /// A switch of lazy imports, which changes how imports behave from outside the module
    /// that imports (MOD-09).
    LazySwitch,
    /// A name planned for TyPy 0.4 (LIB-13).
    Planned,
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

/// The names planned for TyPy 0.4 (LIB-13).
const PLANNED: [&str; 10] = [
    "byteorder",
    "getrecursionlimit",
    "hexversion",
    "maxsize",
    "maxunicode",
    "platform",
    "setrecursionlimit",
    "stdin",
    "version",
    "version_info",
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
        "exit" => SysName::Exit,
        _ if LAZY_SWITCHES.contains(&name) => SysName::LazySwitch,
        _ if PLANNED.contains(&name) => SysName::Planned,
        _ if OTHERS.binary_search(&name).is_ok() => SysName::Unprovided,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{OTHERS, SysName, SysValue, sys_name};

    #[test]
    fn each_kind_of_name_is_found() {
        assert!(OTHERS.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(sys_name("argv"), Some(SysName::Value(SysValue::Argv)));
        assert_eq!(sys_name("stdout"), Some(SysName::Value(SysValue::Stdout)));
        assert_eq!(sys_name("stderr"), Some(SysName::Value(SysValue::Stderr)));
        assert_eq!(sys_name("exit"), Some(SysName::Exit));
        assert_eq!(sys_name("set_lazy_imports"), Some(SysName::LazySwitch));
        assert_eq!(sys_name("maxsize"), Some(SysName::Planned));
        assert_eq!(sys_name("settrace"), Some(SysName::Unprovided));
        assert_eq!(sys_name("argvs"), None);
    }
}
