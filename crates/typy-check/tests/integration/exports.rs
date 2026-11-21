//! Tests of the functions a library exports, through the public API: the names `__all__`
//! lists, each a function with C types, and E0429 for each that cannot be one (FFI-05,
//! ADR-0032, ADR-0044); and the release of its version that names its files (CLI-13,
//! ADR-0045).

use camino::Utf8PathBuf;
use typy_check::{Export, TypeChecker, TypedProgram};
use typy_diagnostics::Diagnostics;
use typy_source::{Project, ProjectSettings};

/// Returns a new, empty directory for one project.
fn fresh_root() -> Utf8PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let root = base.join(format!(
        "typy-exports-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// Checks `source` as the module `tools.py` of a library build.
fn library(source: &str) -> Result<TypedProgram, Diagnostics> {
    let root = fresh_root();
    std::fs::write(root.join("tools.py"), source).unwrap();
    let mut settings = ProjectSettings::new(root.clone());
    settings.library = true;
    let project = Project::load(&root.join("tools.py"), settings).unwrap();
    let checked = TypeChecker::check(&project);
    let _ = std::fs::remove_dir_all(&root);
    checked
}

/// Returns the message of each diagnostic of a library build of `source`, each with its
/// code and rule.
fn refusals(source: &str) -> Vec<String> {
    let diagnostics = library(source).expect_err("the library is refused");
    diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                "{} {} {}",
                diagnostic.code(),
                diagnostic.rule().map_or("", |rule| rule.as_str()),
                diagnostic.message()
            )
        })
        .collect()
}

const FUNCTIONS: &str = "\
def area(width: float, height: float) -> float:
    return width * height


def count(limit: int, odd: bool) -> int:
    return len([n for n in range(limit) if (n % 2 == 1) == odd])


def report() -> None:
    print('report')


def label(n: int) -> str:
    return str(n)
";

#[test]
fn the_exports_are_the_functions_all_lists_in_its_order() {
    let program = library(&format!(
        "{FUNCTIONS}\n__all__ = ['count', 'area', 'report']\n"
    ))
    .expect("the library is accepted");
    let exports = program.exports.clone().expect("a library has exports");
    let names: Vec<&str> = exports.iter().map(|export| export.name.as_str()).collect();
    assert_eq!(names, ["count", "area", "report"]);
    let Export { function, .. } = &exports[1];
    assert_eq!(program.function(*function).name, "area");
}

#[test]
fn a_tuple_or_an_annotated_all_lists_the_exports_too() {
    for binding in [
        "__all__ = ('area',)",
        "__all__: list[str] = ['area']",
        "__all__: tuple[str, ...] = ('area',)",
    ] {
        let program = library(&format!("{FUNCTIONS}\n{binding}\n")).expect(binding);
        assert_eq!(
            program.exports.map(|exports| exports.len()),
            Some(1),
            "{binding}"
        );
    }
}

#[test]
fn a_program_that_is_not_a_library_has_no_exports() {
    let root = fresh_root();
    std::fs::write(root.join("app.py"), FUNCTIONS).unwrap();
    let project = Project::load(&root.join("app.py"), ProjectSettings::new(root.clone())).unwrap();
    let program = TypeChecker::check(&project).unwrap();
    assert_eq!(program.exports, None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn each_name_that_cannot_be_exported_is_e0429() {
    for (all, message) in [
        (
            "['label']",
            "the result of `label` is `str`, which has no C type",
        ),
        (
            "['missing']",
            "`__all__` lists `missing`, which is not a function the module defines",
        ),
        ("['area', 'area']", "`__all__` lists `area` twice"),
        (
            "['read']",
            "`read` is a C function the library itself calls",
        ),
        (
            "['sqrt']",
            "`sqrt` is a C function the library itself calls",
        ),
        (
            "['main']",
            "`main` is a C function the library itself calls",
        ),
        (
            "['typy_area']",
            "`typy_area` is a name C keeps for its implementation or TyPy",
        ),
        ("['__area']", "`__area` is a name C keeps"),
        ("['_Area']", "`_Area` is a name C keeps"),
        ("['caf\u{e9}']", "`caf\u{e9}` is not an ASCII name"),
        (
            "[1]",
            "a library's `__all__` is a list or tuple of string literals",
        ),
        (
            "names",
            "a library's `__all__` is a list or tuple of string literals",
        ),
    ] {
        let source = format!("{FUNCTIONS}\nnames = ['area']\n__all__ = {all}\n");
        let found = refusals(&source);
        assert!(
            found
                .iter()
                .any(|each| each.starts_with("E0429 FFI-05 ") && each.contains(message)),
            "{all}: {found:?}"
        );
    }
}

#[test]
fn an_export_named_as_a_c_function_the_module_calls_is_e0429() {
    // The export would take the place of the C function in the process, and call itself.
    let found = refusals(
        "import ctypes\n\n__all__ = ['labs']\n\nlibc = ctypes.CDLL(None)\n\
         libc.labs.argtypes = [ctypes.c_long]\nlibc.labs.restype = ctypes.c_long\n\n\n\
         def labs(value: int) -> int:\n    return libc.labs(value)\n",
    );
    assert!(
        found
            .iter()
            .any(|each| each
                .starts_with("E0429 FFI-05 `labs` is a C function the library itself calls")),
        "{found:?}"
    );
}

#[test]
fn a_parameter_without_a_c_type_is_named() {
    let found = refusals(
        "def join(items: list[int], separator: str) -> int:\n    return 0\n\n\n__all__ = ['join']\n",
    );
    assert_eq!(
        found,
        [
            "E0429 FFI-05 the parameter `items` of `join` is `list[int]`, which has no C type; an \
          export takes and returns `int`, `float` and `bool`, and may return `None`"
        ]
    );
}

#[test]
fn every_wrong_name_is_reported_once() {
    let found = refusals(&format!(
        "{FUNCTIONS}\n__all__ = ['label', 'free', 'area', 'none']\n"
    ));
    assert_eq!(found.len(), 3, "{found:?}");
}

#[test]
fn a_generic_function_is_not_an_export() {
    let found =
        refusals("def first[T](value: T) -> T:\n    return value\n\n\n__all__ = ['first']\n");
    assert_eq!(
        found,
        ["E0429 FFI-05 `first` is generic, and an export has one C type for each parameter"]
    );
}

#[test]
fn a_library_lists_its_exports_once_at_module_level() {
    for (source, message) in [
        (
            FUNCTIONS.to_owned(),
            "lists the functions it exports in `__all__`, and this one has none",
        ),
        (
            format!("{FUNCTIONS}\n__all__ = ['area']\n__all__ = ['count']\n"),
            "binds `__all__` once, and this binds it again",
        ),
        (
            format!("{FUNCTIONS}\n__all__ = ['area']\n__all__ += ['count']\n"),
            "binds `__all__` once, and this binds it again",
        ),
    ] {
        let found = refusals(&source);
        assert!(found.iter().any(|each| each.contains(message)), "{found:?}");
    }
}

#[test]
fn the_allocators_functions_are_not_exports() {
    let root = fresh_root();
    std::fs::write(
        root.join("tools.py"),
        "def pool_get(size: int) -> int:\n    return size\n\n\n__all__ = ['pool_get']\n",
    )
    .unwrap();
    std::fs::write(
        root.join("typy.toml"),
        "[allocator]\nallocate = \"pool_get\"\nreallocate = \"pool_resize\"\nfree = \"pool_put\"\n",
    )
    .unwrap();
    let overrides = typy_source::SettingOverrides {
        library: true,
        ..typy_source::SettingOverrides::default()
    };
    let settings = ProjectSettings::read(&root, overrides).unwrap();
    let project = Project::load(&root.join("tools.py"), settings).unwrap();
    let diagnostics = TypeChecker::check(&project).unwrap_err();
    let message = diagnostics.iter().next().unwrap().message().to_owned();
    assert!(
        message.contains("`pool_get` is a C function the library itself calls"),
        "{message}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_library_module_is_named_after_its_file() {
    let program = library(&format!(
        "{FUNCTIONS}\n__all__ = ['area']\nNAME = __name__\n"
    ))
    .unwrap();
    let module = program
        .modules
        .iter()
        .find(|module| module.body == program.entry)
        .unwrap();
    assert_eq!(module.name, "tools");
}

/// Checks `source` as a library whose `typy.toml` holds `settings`.
fn library_with_settings(source: &str, settings: &str) -> Result<TypedProgram, Diagnostics> {
    let root = fresh_root();
    std::fs::write(root.join("tools.py"), source).unwrap();
    std::fs::write(root.join("typy.toml"), settings).unwrap();
    let overrides = typy_source::SettingOverrides {
        library: true,
        ..typy_source::SettingOverrides::default()
    };
    let project = Project::load(
        &root.join("tools.py"),
        ProjectSettings::read(&root, overrides).unwrap(),
    )
    .unwrap();
    let checked = TypeChecker::check(&project);
    let _ = std::fs::remove_dir_all(&root);
    checked
}

#[test]
fn the_release_of_version_names_the_library() {
    for (version, release) in [
        ("5.0", "5.0"),
        ("5.0rc1", "5.0"),
        ("v2", "2"),
        ("2024.11.1", "2024.11.1"),
    ] {
        let source = format!("{FUNCTIONS}\n__all__ = ['area']\n__version__ = '{version}'\n");
        let program = library(&source).unwrap();
        assert_eq!(
            program.library_version.as_deref(),
            Some(release),
            "{version}"
        );
    }
    let unversioned = library(&format!("{FUNCTIONS}\n__all__ = ['area']\n")).unwrap();
    assert_eq!(unversioned.library_version, None);
}

#[test]
fn typy_toml_versions_a_module_without_a_version() {
    let source = format!("{FUNCTIONS}\n__all__ = ['area']\n");
    let program = library_with_settings(&source, "version = \"3.1.4\"\n").unwrap();
    assert_eq!(program.library_version.as_deref(), Some("3.1.4"));
    let same = format!("{source}__version__ = '3.1.4'\n");
    let program = library_with_settings(&same, "version = \"3.1.4\"\n").unwrap();
    assert_eq!(program.library_version.as_deref(), Some("3.1.4"));
}

#[test]
fn a_version_that_cannot_name_the_library_is_e0429() {
    let base = format!("{FUNCTIONS}\n__all__ = ['area']\n");
    for (version, message) in [
        (
            "__version__ = 'latest'\n",
            "`__version__` is \"latest\", which does not start with a release",
        ),
        (
            "VERSION = '1'\n__version__ = VERSION\n",
            "a library's `__version__` is a string literal",
        ),
        (
            "__version__ = '1.0'\n__version__ = '1.1'\n",
            "binds `__version__` once",
        ),
    ] {
        let found = refusals(&format!("{base}{version}"));
        assert!(
            found
                .iter()
                .any(|each| each.starts_with("E0429 CLI-13 ") && each.contains(message)),
            "{version}: {found:?}"
        );
    }
    let differs = library_with_settings(
        &format!("{base}__version__ = '5.0'\n"),
        "version = \"5.1\"\n",
    )
    .unwrap_err();
    let message = differs.iter().next().unwrap().message().to_owned();
    assert_eq!(
        message,
        "`__version__` is \"5.0\" and `version` in typy.toml is \"5.1\"; a library has one version"
    );
}

#[test]
fn an_export_and_a_version_that_are_wrong_are_both_reported() {
    let found = refusals(&format!(
        "{FUNCTIONS}\n__all__ = ['label']\n__version__ = 'x'\n"
    ));
    assert_eq!(found.len(), 2, "{found:?}");
}
