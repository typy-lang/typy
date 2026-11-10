# TyPy

TyPy is an independent implementation of Python that compiles a strict,
typed subset of standard Python ahead of time to small native executables
through LLVM.

Documentation: [docs/user/index.md](docs/user/index.md) for users, and
[docs/index.md](docs/index.md) for the engineering documents. Source:
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

TyPy 0.2.0 checks, builds and runs programs made of module-level functions,
classes and statements over `int`, `float`, `str`, `bytes`, `bool`, `None`,
lists, tuples, dicts, sets and optional values, with comprehensions,
f-strings and the format specification language, slices, unpacking, `if`,
`while` and `for`. Programs print what CPython prints, including the last
line of an uncaught exception. Exceptions, imports, inheritance and `ctypes`
come in later releases; the [language guide](docs/user/guide/index.md)
lists what each release accepts.

## Quick start

TyPy runs on x86_64 Linux with glibc 2.39 or later, such as Ubuntu 24.04
or Debian 13, and needs a C toolchain (`cc`). Download the archive from the
[0.2.0 release](https://github.com/typy-lang/typy/releases/tag/v0.2.0),
unpack it, and run a program:

```sh
tar -xzf typy-0.2.0-x86_64-unknown-linux-gnu.tar.gz
export PATH="$PWD/typy-0.2.0-x86_64-unknown-linux-gnu/bin:$PATH"
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
