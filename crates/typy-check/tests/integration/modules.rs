//! Tests of checking programs of several modules through the public API (SMT-07, MOD-01,
//! MOD-02, MOD-10, ADR-0015).

use camino::Utf8PathBuf;
use typy_source::{Project, ProjectSettings};

use typy_check::TypeChecker;

/// A project of `files` in a new directory under the system's temporary directory.
struct Files {
    root: Utf8PathBuf,
}

impl Files {
    fn new(files: &[(&str, &str)]) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
        let root = base.join(format!("typy-modules-{}-{number}", std::process::id()));
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        Self { root }
    }

    fn project(&self) -> Project {
        Project::load(
            &self.root.join("app.py"),
            ProjectSettings::new(self.root.clone()),
        )
        .unwrap_or_else(|failure| panic!("does not load: {:?}", failure.diagnostics))
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Returns the typed dump of an accepted program.
fn accepted(files: &[(&str, &str)]) -> String {
    let files = Files::new(files);
    let project = files.project();
    let program = TypeChecker::check(&project)
        .unwrap_or_else(|diagnostics| panic!("rejected: {diagnostics:?}"));
    typy_check::dump(&program, &project)
}

/// Returns the codes of a rejected program.
fn rejected(files: &[(&str, &str)]) -> Vec<String> {
    let files = Files::new(files);
    let project = files.project();
    let diagnostics = TypeChecker::check(&project).unwrap_err();
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code().to_string())
        .collect()
}

const SHAPES: &str = "\
NAME = 'shapes'
count: int | None = None


class Point:
    def __init__(self, x: int) -> None:
        self.x = x


class ShapeError(Exception):
    pass


def area(p: Point) -> int:
    return p.x
";

#[test]
fn imported_names_resolve_to_their_modules() {
    let dump = accepted(&[
        (
            "app.py",
            "\
import shapes
import tools.text as text
from shapes import Point, area, NAME
from tools import text as words


def measure(p: shapes.Point) -> int:
    from shapes import area as size
    import tools
    print(tools.text)
    return size(p) + len(tools.text.LOUD)


p = Point(2)
print(area(p), shapes.area(shapes.Point(3)), NAME, shapes.NAME, measure(p))
print(shapes, text is words, text.__name__, shapes.Point)
shapes.NAME = 'renamed'
shapes.count = 1
try:
    shapes.area(p)
except shapes.ShapeError:
    pass
",
        ),
        ("shapes.py", SHAPES),
        ("tools/__init__.py", ""),
        ("tools/text.py", "LOUD = 'loud'\n"),
    ]);
    for node in [
        "Import shapes binds shapes = shapes",
        "Import tools tools.text binds text = tools.text",
        "Import shapes binds NAME = shapes.NAME",
        "Import tools tools.text binds words = tools.text",
        "ModuleAttribute shapes.NAME",
        "Submodule tools.text",
    ] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn imports_that_cannot_resolve_give_their_codes() {
    let cases: &[(&str, &[&str])] = &[
        ("from shapes import missing\n", &["E0425"]),
        ("import shapes\nprint(shapes.missing)\n", &["E0425"]),
        ("import shapes\nhandler: int = shapes.area\n", &["E0402"]),
        (
            "import shapes\nx: shapes.Missing = shapes.Point(1)\n",
            &["E0425"],
        ),
        ("import shapes\nshapes.NAME = 1\n", &["E0402"]),
        ("import shapes\nshapes.area = 1\n", &["E0402"]),
        ("import shapes\nshapes.NAME()\n", &["E0401"]),
        ("import shapes\nshapes.missing()\n", &["E0425"]),
        ("import shapes\nshapes.NAME += 1\n", &["E0401"]),
        ("import time\n", &["E0201"]),
        ("import hashlib\n", &["E0206"]),
        ("import collections\n", &["E0201"]),
        ("from collections.abc import Sized\n", &["E0201"]),
        ("from types import FrameType\n", &["E0425"]),
        ("import typing\nprint(typing)\n", &["E0202"]),
        (
            "from typing import Iterator\nx: Iterator = iter([1])\n",
            &["E0408"],
        ),
        (
            "from types import ModuleType\ndef f(m: ModuleType) -> None:\n    print(m.x)\n",
            &["E0412"],
        ),
        (
            "from typing import Iterable\ndef f(xs: Iterable[int]) -> bool:\n    return not xs\n",
            &["E0304"],
        ),
        (
            "from typing import Iterable\ndef f(xs: Iterable[int]) -> bool:\n    return bool(xs)\n",
            &["E0304"],
        ),
        ("import random\n", &["E0201"]),
        ("import shapes\nshapes.Point = 1\n", &["E0402"]),
        (
            "def f() -> None:\n    from shapes import missing\n",
            &["E0425"],
        ),
        ("import shapes\nx: shapes.Point.inner = 1\n", &["E0205"]),
        ("y = 1\nx: y.Point = 1\n", &["E0205"]),
    ];
    for (source, expected) in cases {
        let codes = rejected(&[("app.py", source), ("shapes.py", SHAPES)]);
        assert_eq!(&codes, expected, "{source}");
    }
}

#[test]
fn a_function_importing_a_module_that_imports_its_own_waits_for_it() {
    accepted(&[
        ("app.py", "import first\n"),
        (
            "first.py",
            "import second\nVALUE = 1\n\n\ndef go() -> None:\n    pass\n",
        ),
        (
            "second.py",
            "def later() -> None:\n    import first\n    print(first.VALUE)\n    first.go()\n    from first import VALUE\n",
        ),
    ]);
}

#[test]
fn an_init_importing_a_module_that_imports_its_own_gives_e0426() {
    let codes = rejected(&[
        ("app.py", "import first\n"),
        ("first.py", "import second\nVALUE = 1\n"),
        (
            "second.py",
            "class Counter:\n    def __init__(self) -> None:\n        import first\n        self.count = first.VALUE\n",
        ),
    ]);
    assert_eq!(codes, ["E0426"]);
}

#[test]
fn a_package_importing_a_name_it_lacks_gives_e0425() {
    let codes = rejected(&[
        ("app.py", "import pkg\n"),
        ("pkg/__init__.py", "from . import missing\n"),
    ]);
    assert_eq!(codes, ["E0425"]);
}

#[test]
fn a_variable_another_module_assigns_is_not_narrowed() {
    let codes = rejected(&[
        ("app.py", "import config\nconfig.level = None\n"),
        (
            "config.py",
            "level: int | None = 1\nif level is not None:\n    print(level + 1)\n",
        ),
    ]);
    assert_eq!(codes, ["E0401"]);
}

#[test]
fn typing_collections_abc_and_types_give_annotations() {
    let dump = accepted(&[
        (
            "app.py",
            "\
from collections.abc import Iterable
import typing
import collections.abc
import types as kinds
from types import TracebackType, ModuleType
import shapes


def total(values: Iterable[int], more: typing.Iterable[int]) -> int:
    from typing import Iterator
    steps: Iterator[int] = iter(more)
    return sum(values) + next(steps, 0)


def first(values: collections.abc.Iterator[str], trace: kinds.TracebackType | None) -> str:
    return next(values, 'none')


def show(module: ModuleType, trace: TracebackType | None) -> None:
    print(module, module is shapes)


print(total([1], (2, 3)), total([], range(2)), first(iter(['a']), None))
show(shapes, None)
",
        ),
        ("shapes.py", SHAPES),
    ]);
    for node in ["AsIterable", "Iter : Iterator[int]"] {
        assert!(dump.contains(node), "{node} missing from\n{dump}");
    }
}

#[test]
fn class_values_of_another_module_are_read_and_assigned_there() {
    let settings = "class Limits:\n    retries = 3\n    timeout: float = 1.5\n";
    let dump = accepted(&[
        ("settings.py", settings),
        (
            "app.py",
            "import settings\nfrom settings import Limits\n\
             class Strict(Limits):\n    retries = 1\n\
             def bump() -> None:\n    Limits.retries = Limits.retries + 1\n\
             Limits.retries += 2\nbump()\n\
             print(Limits.retries, settings.Limits.retries, Limits().retries, Limits.timeout, Strict().timeout)\n",
        ),
    ]);
    assert!(dump.contains("Limits.retries"), "{dump}");
    assert_eq!(
        rejected(&[
            ("settings.py", settings),
            (
                "app.py",
                "from settings import Limits\nLimits.timeout = 2.0\n"
            ),
        ]),
        ["E0414"]
    );
}

#[test]
fn a_class_method_inherited_from_another_module_gets_its_copy() {
    let shapes = "import math\nclass Shape:\n    @classmethod\n    def make(cls) -> 'Shape':\n        print(math.pi)\n        return cls()\n";
    let dump = accepted(&[
        ("shapes.py", shapes),
        (
            "app.py",
            "from shapes import Shape\nclass Square(Shape):\n    pass\nprint(Shape.make(), Square.make())\n",
        ),
    ]);
    assert!(dump.contains("Square.make"), "{dump}");
}

#[test]
fn a_dataclass_derives_from_one_named_through_its_module() {
    let shapes = "from dataclasses import dataclass\n@dataclass\nclass Point:\n    x: int\n";
    accepted(&[
        (
            "app.py",
            "from dataclasses import dataclass\nimport shapes as s\n@dataclass\nclass Labeled(s.Point):\n    label: str = 'p'\nprint(Labeled(1).x)\n",
        ),
        ("shapes.py", shapes),
    ]);
    let frozen = rejected(&[
        (
            "app.py",
            "from dataclasses import dataclass\nimport shapes\n@dataclass(frozen=True)\nclass Labeled(shapes.Point):\n    label: str = 'p'\n",
        ),
        ("shapes.py", shapes),
    ]);
    assert_eq!(frozen, vec!["E0404".to_string()]);
}

#[test]
fn a_dataclass_derives_from_one_of_another_module() {
    let base = "from dataclasses import dataclass\n@dataclass\nclass Base:\n    x: int\n";
    let dump = accepted(&[
        ("base.py", base),
        (
            "app.py",
            "from dataclasses import dataclass\nfrom base import Base\n@dataclass\nclass Child(Base):\n    y: int = 2\nprint(Child(1), Child(1) == Child(1, 2))\n",
        ),
    ]);
    assert!(dump.contains("Child.__init__"), "{dump}");
}

#[test]
fn an_augmented_module_variable_of_another_module_keeps_its_type() {
    let found = rejected(&[
        ("app.py", "import limits\nlimits.LIMIT += 1.5\n"),
        ("limits.py", "LIMIT = 1\n"),
    ]);
    assert!(found.contains(&"E0402".to_string()), "{found:?}");
}

#[test]
fn version_info_of_another_module_is_its_own_variable() {
    let dump = accepted(&[
        (
            "app.py",
            "import util\nprint(util.version_info == (1, 2))\n",
        ),
        ("util.py", "version_info = (1, 2)\n"),
    ]);
    assert!(dump.contains("version_info"), "{dump}");
}

#[test]
fn a_module_of_the_program_hides_a_standard_module_of_its_name() {
    // CPython finds `typing.py` beside the program before its own `typing`.
    accepted(&[
        ("app.py", "from typing import helper\nprint(helper())\n"),
        ("typing.py", "def helper() -> int:\n    return 1\n"),
    ]);
}

/// A module that defines a class `Shape`.
const SHAPES_A: &str =
    "class Shape:\n    def __init__(self, n: int) -> None:\n        self.n = n\n";
/// Another module that defines a class `Shape`.
const SHAPES_B: &str =
    "class Shape:\n    def __init__(self, n: int) -> None:\n        self.n = n * 10\n";

#[test]
fn a_class_an_import_in_a_block_binds_is_read_as_a_value_unless_a_local_hides_it() {
    let dump = accepted(&[
        (
            "app.py",
            "import sys\nif len(sys.argv) < 5:\n    from shapes_a import Shape\n\
             made = Shape\nprint(made(2).n)\n\
             def shadow() -> int:\n    Shape = 3\n    return Shape\nprint(shadow())\n",
        ),
        ("shapes_a.py", SHAPES_A),
    ]);
    assert!(dump.contains("Local Shape : int"), "{dump}");
}

#[test]
fn a_class_two_imports_bind_read_as_a_value_is_planned() {
    let found = rejected(&[
        (
            "app.py",
            "import sys\nif len(sys.argv) > 5:\n    from shapes_a import Shape\n\
             else:\n    from shapes_b import Shape\nmade = Shape\n",
        ),
        ("shapes_a.py", SHAPES_A),
        ("shapes_b.py", SHAPES_B),
    ]);
    assert_eq!(found, ["E0201"]);
}
