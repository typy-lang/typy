//! Tests of checking `format % values` through the public API (LIB-10, LIB-11): each format
//! that does not fit its values is refused with CPython's message.

use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings};

/// Returns each diagnostic of `source` as its code and message.
fn diagnostics(source: &str) -> Vec<(String, String)> {
    let directory = tempfile_dir();
    std::fs::write(directory.join("app.py"), source).unwrap();
    let project = Project::load(
        &directory.join("app.py"),
        ProjectSettings::new(directory.clone()),
    )
    .unwrap();
    let found = match TypeChecker::check(&project) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code().to_string(),
                    diagnostic.message().to_owned(),
                )
            })
            .collect(),
    };
    let _ = std::fs::remove_dir_all(&directory);
    found
}

fn tempfile_dir() -> camino::Utf8PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let directory = base.join(format!(
        "typy-percent-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn formats_that_fit_their_values_are_accepted() {
    for source in [
        "print('%s %d %f %x %c %c' % ('a', 1, 2.0, 255, 65, 'b'))\n",
        "print('%d %i %u %e %g' % (True, 1.5, 2, 3, True))\n",
        "print('%*.*f %-*s' % (8, 2, 1.5, 4, 'ab'))\n",
        "table = {'a': 1}\nprint('%(a)d %(a)s %(a)5.1f' % table)\n",
        "table = {'a': 1}\nprint('%s %(a)s' % table)\nprint('%(a)s' % table)\n",
        "print('100%%' % ())\nprint('%s' % [1])\nprint('%s' % {'a': 1})\nprint('x' % {'a': 1})\n",
        "value: int | None = None\nprint('%s %r' % (value, value))\n",
        "print('%ld %hd %Lx %5.s|' % (1, 2, 255, 'abc'))\n",
        "print('%d' % 1, '%x' % 255, '%f' % 1, '%c' % 65)\n",
        "print(\"%(it's)s %(a\\\\b)s\" % {\"it's\": 'x', 'a\\\\b': 'y'})\n",
    ] {
        assert_eq!(diagnostics(source), [], "{source}");
    }
}

#[test]
fn formats_that_do_not_fit_give_cpythons_message() {
    for (source, message) in [
        (
            "print('%d %d' % (1,))\n",
            "not enough arguments for format string (got 1)",
        ),
        (
            "print('%d' % (1, 2))\n",
            "not all arguments converted during string formatting (required 1, got 2)",
        ),
        (
            "print('x' % 1)\n",
            "not all arguments converted during string formatting (required 0, got 1)",
        ),
        ("print('%' % 1)\n", "stray % at position 0"),
        (
            "print('%(a' % {'a': 1})\n",
            "stray % or incomplete format key at position 0",
        ),
        ("print('%y' % 1)\n", "unsupported format %y at position 0"),
        (
            "print('%d' % 'a')\n",
            "format argument: %d requires a real number, not str",
        ),
        (
            "print('%s %x' % (1, 1.5))\n",
            "format argument 2: %x requires an integer, not float",
        ),
        (
            "print('%f' % [1])\n",
            "format argument: %f requires a real number, not list",
        ),
        (
            "print('%c' % 1.5)\n",
            "format argument: %c requires an integer or a unicode character, not float",
        ),
        (
            "print('%*d' % ('a', 1))\n",
            "format argument 1: * requires int, not str",
        ),
        (
            "print('%(a)s' % (1,))\n",
            "format requires a mapping, not tuple",
        ),
        (
            "print('%(a)*d' % {'a': 1})\n",
            "* cannot be used with a parenthesised mapping key at position 0",
        ),
        (
            "print('%(a)s %d' % {'a': 1})\n",
            "format requires a parenthesised mapping key at position 6",
        ),
        (
            "print('%d %(a)s' % {'a': 1})\n",
            "format argument: %d requires a real number, not dict",
        ),
        (
            "print(\"%(it's)d\" % {\"it's\": 'a'})\n",
            "format argument \"it's\": %d requires a real number, not str",
        ),
        (
            "print('%5!' % 1)\n",
            "stray % at position 0 or unexpected format character '!' at position 2",
        ),
        (
            "print('%\\n' % 1)\n",
            "stray % at position 0 or unexpected format character U+000A at position 1",
        ),
        (
            "print('%\u{e9}' % 1)\n",
            "stray % at position 0 or unexpected format character '\u{e9}' (U+00E9) at position 1",
        ),
        (
            "print('%(a\\\\b)d' % {'a\\\\b': 'x'})\n",
            "format argument 'a\\\\b': %d requires a real number, not str",
        ),
        (
            "print('%o' % 'a')\n",
            "format argument: %o requires an integer, not str",
        ),
        (
            "def show(value: int | None) -> None:\n    print('%d' % value)\n",
            "format argument: %d requires a real number, not NoneType",
        ),
    ] {
        assert_eq!(
            diagnostics(source),
            [("E0427".to_owned(), message.to_owned())],
            "{source}"
        );
    }
}

#[test]
fn values_that_already_failed_give_no_second_error() {
    let found = diagnostics("print('%d' % missing)\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0205");
}

#[test]
fn formats_planned_for_0_4_give_e0201() {
    for source in [
        "template = '%s'\nprint(template % 1)\n",
        "values: tuple[int, ...] = (1, 2)\nprint('%d %d' % values)\n",
    ] {
        let found = diagnostics(source);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(found[0].0, "E0201", "{source}");
        assert!(found[0].1.ends_with("(LIB-11)"), "{source}: {}", found[0].1);
    }
}
