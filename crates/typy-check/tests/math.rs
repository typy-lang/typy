//! Tests of checking `math` through the public API (LIB-05, LIB-12, ADR-0018): its names,
//! the types its functions take, and the names a later Python floor brings.

use typy_check::TypeChecker;
use typy_source::{Project, ProjectSettings, PythonFloor};

/// Returns each diagnostic of `source`, checked at `floor`, as its code and message.
fn diagnostics_at(floor: PythonFloor, source: &str) -> Vec<(String, String)> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
    let directory = base.join(format!(
        "typy-math-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("app.py"), source).unwrap();
    let mut settings = ProjectSettings::new(directory.clone());
    settings.python_floor = floor;
    let project = Project::load(&directory.join("app.py"), settings).unwrap();
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

fn diagnostics(source: &str) -> Vec<(String, String)> {
    diagnostics_at(PythonFloor::V3_12, source)
}

#[test]
fn every_kind_of_name_of_math_is_accepted() {
    for source in [
        "import math\nprint(math.sqrt(2), math.floor(1.5), math.pi, math.isnan(1))\n",
        "import math as m\nprint(m.log(8, 2), m.log2(8), m.hypot(), m.hypot(3, 4, 5))\n",
        "from math import gcd, lcm, e as euler\nprint(gcd(), lcm(2, 3, 4), euler)\n",
        "from math import isclose\nprint(isclose(1, 2, rel_tol=1.0), isclose(1, 2, abs_tol=2))\n",
        "import math\nprint(math.nextafter(1, 2, steps=2), math.perm(5, None), math.perm(5))\n",
        "import math\nk: int | None = None\nprint(math.perm(5, k), math.nextafter(0, 1, steps=k))\n",
        "import math\nprint(math.frexp(2), math.modf(1.5), math.ldexp(1, True), math.trunc(True))\n",
        "def root(x: float) -> float:\n    from math import sqrt\n    return sqrt(x)\n\n\nprint(root(4.0))\n",
    ] {
        assert_eq!(diagnostics(source), [], "{source}");
    }
}

#[test]
fn arguments_math_refuses_for_every_value_give_cpythons_message() {
    for (source, message) in [
        (
            "import math\nprint(math.sqrt('4'))\n",
            "must be real number, not str",
        ),
        (
            "import math\nprint(math.floor(None))\n",
            "must be real number, not NoneType",
        ),
        (
            "import math\nprint(math.trunc('a'))\n",
            "type str doesn't define __trunc__ method",
        ),
        (
            "import math\nprint(math.factorial(4.0))\n",
            "'float' object cannot be interpreted as an integer",
        ),
        (
            "import math\nprint(math.gcd(1, [2]))\n",
            "'list' object cannot be interpreted as an integer",
        ),
        (
            "import math\nprint(math.ldexp(1, 2.0))\n",
            "Expected an int as second argument to ldexp",
        ),
        (
            "import math\nprint(math.perm(5, 2.0))\n",
            "'float' object cannot be interpreted as an integer",
        ),
        (
            "import math\nprint(math.sqrt())\n",
            "`math.sqrt` takes 1 argument, and 0 are given",
        ),
        (
            "import math\nprint(math.atan2(1))\n",
            "`math.atan2` takes 2 arguments, and 1 is given",
        ),
        (
            "import math\nprint(math.log(1, 2, 3))\n",
            "`math.log` takes from 1 to 2 arguments, and 3 are given",
        ),
        (
            "import math\nprint(math.sqrt(x=1))\n",
            "`math.sqrt` takes no keyword arguments",
        ),
        (
            "import math\nprint(math.isclose(1, 2, tolerance=1))\n",
            "`math.isclose` has no keyword argument `tolerance`",
        ),
    ] {
        assert_eq!(
            diagnostics(source),
            [("E0404".to_owned(), message.to_owned())],
            "{source}"
        );
    }
}

#[test]
fn names_math_does_not_have_at_the_floor_are_e0425() {
    for (floor, source, message) in [
        (
            PythonFloor::V3_12,
            "from math import square\n",
            "module `math` has no name `square`",
        ),
        (
            PythonFloor::V3_12,
            "import math\nprint(math.square)\n",
            "module `math` has no name `square`",
        ),
        (
            PythonFloor::V3_12,
            "import math\nprint(math.fma(1, 2, 3))\n",
            "module `math` has no name `fma` before Python 3.13",
        ),
        (
            PythonFloor::V3_14,
            "from math import fmax\n",
            "module `math` has no name `fmax` before Python 3.15",
        ),
    ] {
        assert_eq!(
            diagnostics_at(floor, source),
            [("E0425".to_owned(), message.to_owned())],
            "{source}"
        );
    }
    assert_eq!(
        diagnostics_at(
            PythonFloor::V3_13,
            "import math\nprint(math.fma(1, 2, 3))\n"
        ),
        []
    );
    assert_eq!(
        diagnostics_at(
            PythonFloor::V3_15,
            "from math import fmax, fmin, signbit\nprint(fmax(1, 2))\n"
        ),
        []
    );
}

#[test]
fn other_uses_of_math_give_their_codes() {
    for (source, code) in [
        ("import math\nprint(math.fsum([1.0]))\n", "E0201"),
        ("from math import prod\n", "E0201"),
        ("import math\nf = math.sqrt\n", "E0201"),
        ("from math import sqrt\nf = sqrt\n", "E0201"),
        ("import math\nprint(math.pi())\n", "E0401"),
        ("import math.special\n", "E0206"),
    ] {
        let found = diagnostics(source);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert_eq!(found[0].0, code, "{source}: {found:?}");
    }
}
