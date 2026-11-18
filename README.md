# TyPy

TyPy is an independent implementation of Python that compiles a strict,
typed subset of standard Python ahead of time to small native executables
through LLVM.

Documentation: <https://typy-lang.org>, for users and for the engineering
documents of the latest release, built from [docs/](docs/index.md). Source:
<https://github.com/typy-lang/typy>.

## Features

These are the goals of TyPy 1.0, as the
[requirements specification](docs/requirements/srs.md) states them:

- Source is ordinary Python. Every program TyPy accepts also runs under
  CPython, from 3.12 up, with the same output and exit status.
- TyPy refuses a program it cannot compile with CPython's meaning, and the
  diagnostic names the rule, such as `EXP-04`. It never changes what valid
  Python means.
- Output is one native executable with no interpreter and no garbage
  collector pauses. A hello world built statically with musl is under
  100 KB.
- Memory is managed by reference counting that the compiler optimises,
  with cycle collection only for types that can form a cycle.
- `int` has no size limit, as in Python, and values that fit in 63 bits
  stay inline.
- C libraries are called through the standard `ctypes` module, and
  functions can be exported to C.

## Status

TyPy 0.6.0 runs on x86_64 and 64-bit Arm (AArch64) Linux and builds
programs for either, with `typy build --target`. It checks, builds and
runs programs made of modules, functions,
classes with inheritance, dataclasses and statements over `int`, `float`,
`str`, `bytes`, `bool`, `None`, lists, tuples, dicts, sets, optional values
and unions, with comprehensions, assignment expressions, f-strings, `%`
formatting, slices and unpacking; `if`, `while`, `for`, `match`, `try`,
`with`, `raise` and `assert`; PEP 695 generic functions, classes and
methods, protocols, `Final` and `type` aliases; exceptions with CPython's
classes and messages, including classes of the program's own; imports of
the program's modules, packages and PEP 810 lazy imports; the `math` and
`sys` modules, `input`, and text and binary files through `open`; and C
libraries through `ctypes`, with objects, pointers, arrays and structures.
`typy build --lib` builds a module as a static or shared C library with a
C header and a Python binding. Programs print what CPython prints,
including the traceback of an uncaught exception. Closures and generators
come in later releases; the
[language reference](https://typy-lang.org/reference/) lists what each
release accepts.

## Quick start

TyPy runs on x86_64 and AArch64 Linux with glibc 2.39 or later, such as
Ubuntu 24.04 or Debian 13, and needs a C toolchain (`cc`). Download the
archive for your machine, `x86_64` or `aarch64`, which `uname -m` prints,
from the
[0.6.0 release](https://github.com/typy-lang/typy/releases/tag/v0.6.0),
unpack it, and run a program:

```sh
tar -xzf "typy-0.6.0-$(uname -m)-unknown-linux-gnu.tar.gz"
export PATH="$PWD/typy-0.6.0-$(uname -m)-unknown-linux-gnu/bin:$PATH"
printf 'print("hello", 1 + 2)\n' > hello.py
typy run hello.py
```

`typy build hello.py` writes the executable `hello` beside the file, and
`typy check hello.py` reports problems without building.

## Building from source

You need Docker with Compose v2, GNU Make and Git. Every other tool runs in
containers.

```sh
make hooks
make build
make shell
```

`make build` builds the compiler in the dev container, and `make shell`
opens a shell there, where `$CARGO_TARGET_DIR/debug/typy` is the compiler.
`make check` runs the quality gate. The
[development guide](docs/development/development-guide.md) lists every
command.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

TyPy is licensed under either of the
[Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option. Unless you state otherwise, any
contribution you submit for inclusion in TyPy is licensed in the same way,
without any additional terms or conditions.

Parts of the runtime are derived from CPython and the Unicode Character
Database and keep their own licences, which
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) holds.
