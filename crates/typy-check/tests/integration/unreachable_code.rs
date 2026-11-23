//! Code that cannot run is neither checked nor compiled: a statement after one that leaves
//! its block, and a branch an `isinstance` test, a class pattern, or the constant `True` or
//! `False` rules out (TYP-34, ADR-0111).

use crate::common::{app, messages};

#[test]
fn typ_34_code_that_cannot_run_is_not_checked() {
    let accepted = [
        // After `return`, `raise`, `continue` and `break`.
        "def f(x: int) -> int:\n    return x\n    print(x.upper())\n\n\nprint(f(1))\n",
        "def f(x: int) -> int:\n    raise ValueError(x)\n    return x.upper()\n",
        "def f(n: int) -> int:\n    total = 0\n    for i in range(n):\n        continue\n        \
         total = total.upper()\n    return total\n\n\nprint(f(3))\n",
        "def f(n: int) -> int:\n    while True:\n        break\n        n = n.upper()\n    \
         return n\n\n\nprint(f(3))\n",
        // A branch `isinstance` rules out, either way.
        "def f(x: int) -> int:\n    if isinstance(x, str):\n        return x.upper()\n    \
         return x\n\n\nprint(f(1))\n",
        "def f(x: int) -> int:\n    if isinstance(x, int):\n        return x\n    \
         return x.upper()\n\n\nprint(f(1))\n",
        // A test of `True` decides the branch, so `x` is assigned and `f` returns.
        "if True:\n    x = 1\nprint(x)\n",
        "def f() -> int:\n    if True:\n        return 1\n\n\nprint(f())\n",
        // An `assert` that never holds always raises.
        "def f(x: int) -> int:\n    assert isinstance(x, str)\n    return x.upper()\n",
        // A variable assigned only where the code runs.
        "def f(x: int) -> int:\n    if isinstance(x, str):\n        y = len(x)\n    else:\n        \
         y = x\n    return y\n\n\nprint(f(2))\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn typ_34_code_that_can_run_is_still_checked() {
    let cases = [
        "def f(x: int | str) -> int:\n    if isinstance(x, str):\n        return x.missing()\n    \
         return x\n",
        "def f(x: int, flag: bool) -> int:\n    if flag:\n        return x\n    return x.upper()\n",
        "def f(x: int | None) -> int:\n    if isinstance(x, int):\n        return x\n    \
         return x.upper()\n",
    ];
    for source in cases {
        let found = messages(source);
        assert!(
            found.iter().any(|(code, ..)| code == "E0412"),
            "{source}\n{found:?}"
        );
    }
}
