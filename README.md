# TyPy

TyPy is an independent implementation of Python that compiles a strict,
typed subset of standard Python ahead of time to small native executables
through LLVM.

Documentation: [docs/index.md](docs/index.md). Source:
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

Release 0.1 is being written. The `typy` command checks, builds and runs
programs made of module-level functions and statements over `int`, `float`,
`str`, `bool` and `None`, with `if`, `while`, `for` over `range`, and
`print`. Programs print what CPython prints, including the last line of an
uncaught exception. Classes, containers, modules and `ctypes` come in later
releases.

## Quick start

You need Docker with Compose v2, GNU Make and Git. Every other tool runs in
containers.

```sh
make hooks
make build
make shell
```

`make build` builds the compiler in the dev container, and `make shell`
opens a shell there. In that shell:

```sh
printf 'print("hello", 1 + 2)\n' > hello.py
$CARGO_TARGET_DIR/debug/typy run hello.py
```

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
