//! Calls of builtins, generic functions, `ctypes` functions and type aliases with the arguments
//! each takes.

use crate::common::{app, messages};

#[test]
fn calls_with_the_arguments_their_callees_take_are_accepted() {
    let accepted = [
        "def f[T](*items: T) -> list[T]:\n    return list(items)\n\n\n\
        print(f(*[1], *(2, 3)))\n",
        "def f[T](x: T, y: int) -> T:\n    return x\n\n\nprint(f(1, y=2))\n",
        "def f[T](a: T, *rest: T) -> T:\n    return a\n\n\nprint(f(1, *[2, 3]))\n",
        "from collections.abc import Generator\n\n\n\
        def echo() -> Generator[int, None, None]:\n    yield 1\n    yield 2\n\n\n\
        g = echo()\nprint(next(g))\nprint(g.send(None))\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
}

#[test]
fn a_call_its_callee_does_not_take_is_refused_with_the_reason() {
    let refused = [
        (
            "def f(a: int, b: str) -> None:\n    print(a, b)\n\n\nf(*\"x\".nope(), *[1])\n",
            "E0412",
            "a str has no method `nope` in TyPy",
        ),
        (
            "print(min(key=lambda x: x))\n",
            "E0404",
            "min expected at least 1 argument, got 0: CPython raises `TypeError`",
        ),
        (
            "print(max(1, 2, default=3))\n",
            "E0404",
            "Cannot specify a default for max() with multiple positional arguments: CPython \
            raises `TypeError`",
        ),
        (
            "print(min(1, 2, foo=3))\n",
            "E0404",
            "min() got an unexpected keyword argument 'foo': CPython raises `TypeError`",
        ),
        (
            "print(abs(x=1), len(obj=[1]))\n",
            "E0404",
            "abs() takes no keyword arguments: CPython raises `TypeError`",
        ),
        (
            "print(min(1, 2, key=len))\n",
            "E0404",
            "the argument of `len` must be a str, and this is an int",
        ),
        (
            "import ctypes\n\nlibc = ctypes.CDLL(None)\n\
            libc.strchr.argtypes = [ctypes.c_char_p, ctypes.c_int]\n\
            libc.strchr.restype = ctypes.c_char_p\nprint(libc.strchr(b\"hello\"))\n",
            "E0404",
            "`libc.strchr` takes 2 arguments, as its `argtypes` declare, and 1 was given",
        ),
        (
            "import ctypes\n\nlibc = ctypes.CDLL(None)\n\
            libc.abs.argtypes = [ctypes.c_bool]\nlibc.abs.restype = ctypes.c_int\n\
            flag = ctypes.c_bool(True)\nprint(libc.abs(ctypes.byref(flag)))\n",
            "E0404",
            "argument 1 of `libc.abs` is `byref` of a c_bool, where its `argtypes` take a \
            value with a truth",
        ),
        (
            "import sys\n\nif sys.version_info >= (3, 13):\n    from math import sqrt\n\
            else:\n    from math import sqrt\n\nprint(sqrt(\"x\"))\n",
            "E0404",
            "must be real number, not str",
        ),
        (
            "def f() -> None:\n    type T = int\n    print(T())\n",
            "E0401",
            "'typing.TypeAliasType' object is not callable: CPython raises `TypeError`",
        ),
        (
            "import sys\n\nprint(sys.setrecursionlimit())\n",
            "E0404",
            "`sys.setrecursionlimit` takes 1 positional argument, and 0 are given",
        ),
        (
            "def f() -> None:\n    \
            with open(\"/etc/hostname\", \"rb\", buffering=\"1\".nope()) as handle:\n        \
            print(handle)\n",
            "E0412",
            "a str has no method `nope` in TyPy",
        ),
        (
            "s = slice(1, 2)\nprint(s.nope)\n",
            "E0412",
            "a slice has no attribute `nope`; it has `start`, `stop` and `step`",
        ),
    ];
    for (source, code, message) in refused {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}\n{found:?}"
        );
    }
}

#[test]
fn lib_28_numeric_builtins_take_what_cpython_takes() {
    let accepted = [
        "print(divmod(7, 2), divmod(7.5, 2), divmod(True, 3))\n",
        "print(pow(2, 10), pow(2.0, -1), pow(base=2, exp=3), pow(2, 3, 5), pow(2, 3, mod=None))\n",
        "print(bin(5), hex(True), oct(-8), format(1.5), format(\"x\", \">3\"))\n",
        "from collections.abc import Callable\n\nshow: Callable[[int], str] = hex\nprint(show(1), bin)\n",
    ];
    for source in accepted {
        assert_eq!(app(source), Vec::<String>::new(), "{source}");
    }
    let refused = [
        (
            "print(bin(1.5))\n",
            "E0404",
            "the argument of `bin` must be an int, and this is a float",
        ),
        (
            "print(divmod(\"a\", 2))\n",
            "E0401",
            "`divmod` is not defined for str and int",
        ),
        (
            "print(format(3, 4))\n",
            "E0404",
            "the format specification of `format` must be a str, and this is an int",
        ),
        (
            "print(hex(x=3))\n",
            "E0404",
            "hex() takes no keyword arguments: CPython raises `TypeError`",
        ),
        (
            "print(pow(2, 3, 4.0))\n",
            "E0404",
            "the modulus of a `pow` with a modulus must be an int, and this is a float",
        ),
        (
            "print(pow(2, exp=3, base=1))\n",
            "E0404",
            "argument for pow() given by name ('base') and position (1): CPython raises `TypeError`",
        ),
        (
            "print(pow(2, 3, modulo=5))\n",
            "E0404",
            "pow() got an unexpected keyword argument 'modulo': CPython raises `TypeError`",
        ),
        (
            "print(pow(base=2))\n",
            "E0404",
            "pow() missing required argument 'exp' (pos 2): CPython raises `TypeError`",
        ),
        (
            "print(pow(2, 3, 4, 5))\n",
            "E0404",
            "`pow` takes 2 to 3 arguments, and 4 are given",
        ),
        (
            "print(divmod(1))\n",
            "E0404",
            "`divmod` takes 2 arguments, and 1 are given",
        ),
        (
            "print(pow(2))\n",
            "E0404",
            "pow() missing required argument 'exp' (pos 2): CPython raises `TypeError`",
        ),
        (
            "print(pow(exp=2))\n",
            "E0404",
            "pow() missing required argument 'base' (pos 1): CPython raises `TypeError`",
        ),
        (
            "n = 3\nprint(pow(2, n))\n",
            "E0302",
            "the exponent of `int ** int` must be a non-negative integer literal, because CPython \
             returns a float for a negative exponent",
        ),
    ];
    for (source, code, message) in refused {
        let found = messages(source);
        assert!(
            found
                .iter()
                .any(|(each, text, _)| each == code && text == message),
            "{source}\n{found:?}"
        );
    }
}
