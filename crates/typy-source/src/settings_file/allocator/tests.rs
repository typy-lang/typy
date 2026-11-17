//! Tests of the `[allocator]` table of `typy.toml` (MEM-08, ADR-0043).

use camino::Utf8PathBuf;

use crate::settings::{Allocator, ProjectSettings};
use crate::settings_file::{SETTINGS_FILE, SettingOverrides};

/// Returns the settings `text` gives, or the message and rule of the first problem.
fn read(text: &str) -> Result<ProjectSettings, (String, String, Option<String>)> {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    std::fs::write(root.join(SETTINGS_FILE), text).unwrap();
    ProjectSettings::read(&root, SettingOverrides::default()).map_err(|failure| {
        let diagnostic = failure.diagnostics.iter().next().unwrap();
        (
            diagnostic.message().to_owned(),
            diagnostic
                .rule()
                .map(|rule| rule.as_str().to_owned())
                .unwrap_or_default(),
            diagnostic.help().map(str::to_owned),
        )
    })
}

#[test]
fn the_table_names_the_three_functions_and_the_library() {
    let settings = read(
        "[allocator]\nallocate = \"mi_malloc\"\nreallocate = \"mi_realloc\"\nfree = \"mi_free\"\n\
         library = \"mimalloc\"\n",
    )
    .unwrap();
    assert_eq!(
        settings.allocator,
        Some(Allocator {
            allocate: "mi_malloc".to_owned(),
            reallocate: "mi_realloc".to_owned(),
            free: "mi_free".to_owned(),
            library: Some("mimalloc".to_owned()),
        })
    );
}

#[test]
fn an_inline_table_without_a_library_is_read() {
    let settings = read(
        "allocator = { allocate = \"pool_get\", reallocate = \"pool_resize\", free = \"_pool_put2\" }\n",
    )
    .unwrap();
    let allocator = settings.allocator.unwrap();
    assert_eq!(allocator.free, "_pool_put2");
    assert_eq!(allocator.library, None);
}

#[test]
fn without_the_table_the_program_keeps_the_c_library_allocator() {
    assert_eq!(read("python-floor = \"3.13\"\n").unwrap().allocator, None);
}

#[test]
fn each_wrong_table_is_e0107_under_mem_08() {
    let all = "allocate = \"a\"\nreallocate = \"r\"\nfree = \"f\"\n";
    let long = "x".repeat(256);
    for (text, message) in [
        (
            "allocator = \"mimalloc\"\n",
            "`allocator` is the string \"mimalloc\", not a table",
        ),
        ("allocator = 3\n", "`allocator` is an integer, not a table"),
        (
            "[allocator]\nallocate = \"a\"\n",
            "`[allocator]` is missing `reallocate` and `free`",
        ),
        (
            "[allocator]\n",
            "`[allocator]` is missing `allocate`, `reallocate` and `free`",
        ),
        (
            &format!("[allocator]\n{all}size = \"8\"\n"),
            "`[allocator]` has no setting `size`",
        ),
        (
            "[allocator]\nallocate = 1\nreallocate = \"r\"\nfree = \"f\"\n",
            "`allocator.allocate` is an integer, not a string",
        ),
        (
            "[allocator]\nallocate = \"9lives\"\nreallocate = \"r\"\nfree = \"f\"\n",
            "`allocator.allocate` is \"9lives\", which is not the name of a C function",
        ),
        (
            "[allocator]\nallocate = \"a\"\nreallocate = \"re-alloc\"\nfree = \"f\"\n",
            "`allocator.reallocate` is \"re-alloc\", which is not the name",
        ),
        (
            "[allocator]\nallocate = \"a\"\nreallocate = \"r\"\nfree = \"\"\n",
            "`allocator.free` is \"\", which is not the name",
        ),
        (
            &format!("[allocator]\nallocate = \"{long}\"\nreallocate = \"r\"\nfree = \"f\"\n"),
            "which is not the name of a C function",
        ),
        (
            "[allocator]\nallocate = \"typy_rt_heap_allocate\"\nreallocate = \"r\"\nfree = \"f\"\n",
            "`allocator.allocate` names `typy_rt_heap_allocate`, but names that start with \
             `typy_` belong to TyPy",
        ),
        (
            "[allocator]\nallocate = \"a\"\nreallocate = \"r\"\nfree = \"__typy_main\"\n",
            "`allocator.free` names `__typy_main`, but names that start with `__typy` belong",
        ),
        (
            "[allocator]\nallocate = \"_TY8__main__4pool\"\nreallocate = \"r\"\nfree = \"f\"\n",
            "names that start with `_TY` belong to TyPy",
        ),
        (
            &format!("[allocator]\n{all}library = \"-lc\"\n"),
            "`allocator.library` is \"-lc\", which is not the name of a library",
        ),
        (
            &format!("[allocator]\n{all}library = \"lib/pool\"\n"),
            "`allocator.library` is \"lib/pool\"",
        ),
        (
            &format!("[allocator]\n{all}library = true\n"),
            "`allocator.library` is a boolean, not a string",
        ),
        (
            "[allocator]\nallocate = \"pool\"\nreallocate = \"resize\"\nfree = \"pool\"\n",
            "`[allocator]` names `pool` for two of its functions",
        ),
    ] {
        let (found, rule, _) = read(text).unwrap_err();
        assert!(found.contains(message), "{text}: {found}");
        assert_eq!(rule, "MEM-08", "{text}");
    }
}

#[test]
fn the_help_names_what_the_table_takes() {
    let (_, _, help) = read("[allocator]\nalloc = \"a\"\n").unwrap_err();
    assert_eq!(
        help.as_deref(),
        Some("the settings are allocate, reallocate, free and library")
    );
    let (_, _, help) = read("[allocator]\nallocate = \"a\"\n").unwrap_err();
    assert_eq!(
        help.as_deref(),
        Some("name all three functions: allocate, reallocate and free")
    );
    let (_, _, help) = read("other = 1\n").unwrap_err();
    assert_eq!(
        help.as_deref(),
        Some(
            "the settings are python-floor, module-paths, allocator, version, library-paths, libraries and target"
        )
    );
}

#[test]
fn library_names_with_versions_and_symbols_are_taken() {
    for name in [
        "jemalloc",
        "tcmalloc_minimal",
        "pool-2.1",
        "c++",
        "_private",
    ] {
        let text = format!(
            "[allocator]\nallocate = \"a\"\nreallocate = \"r\"\nfree = \"f\"\nlibrary = \"{name}\"\n"
        );
        let library = read(&text).unwrap().allocator.unwrap().library;
        assert_eq!(library.as_deref(), Some(name));
    }
}
