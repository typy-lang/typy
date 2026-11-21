# Using TyPy on macOS

TyPy runs on macOS 11 or later on a Mac with Apple silicon, and builds
programs and libraries that run on macOS 11 or later on any Apple silicon
Mac (CLI-15).

## What TyPy accepts

| Item | Rule |
| --- | --- |
| `typy` runs on macOS 11 or later on Apple silicon, and builds for `aarch64-apple-darwin` | CLI-15 |
| A Mac builds for macOS alone, and a Linux machine does not build for macOS; another `--target` is a usage error that lists the host's targets | CLI-08 |
| A shared library is `libNAME.dylib`, or `libNAME.X.dylib` with a link for a release `X.Y`, and a static library `libNAME.a` | CLI-16 |
| `sys.platform` is `"darwin"` | LIB-22 |
| An `OSError` takes macOS's error numbers and the text `strerror` gives, as CPython's does on macOS | EXC-19 |
| A C call passes its arguments as Apple's arm64 convention does, so a C function reads the values CPython passes it | FFI-13 |

## What you need

TyPy links programs with Apple's C compiler and linker, which Xcode's
Command Line Tools install:

```sh
xcode-select --install
```

The tools also provide `dsymutil`, which writes the debug information of
a debug build, and `libtool`, which writes a static library.

## Install from a release archive

Download `typy-0.10.0-aarch64-apple-darwin.tar.gz` and its `.sha256` file
from the [release page](https://github.com/typy-lang/typy/releases), then
check the archive and unpack it:

```sh
archive=typy-0.10.0-aarch64-apple-darwin
shasum -a 256 --check "$archive.tar.gz.sha256"
tar -xzf "$archive.tar.gz"
export PATH="$PWD/$archive/bin:$PATH"
typy --version
```

The archive is signed ad hoc, as Apple's linker signs every program it
links, and is not notarized. A browser marks what it downloads as
quarantined, and macOS then refuses to run `typy`. Clear the mark once,
before you unpack the archive:

```sh
xattr -d com.apple.quarantine "$archive.tar.gz"
```

An archive that `curl` or `gh release download` downloads carries no such
mark.

## Building programs

`typy check`, `typy build` and `typy run` work as on Linux, with the
target `aarch64-apple-darwin`. A debug build writes the program and,
beside it, `NAME.dSYM`, the debug information that `lldb` reads; a
release build writes the program alone. A program has no static form on
macOS, so the musl target has no macOS counterpart.

A program that calls C through `ctypes` names a library by the file name
macOS gives it, such as `libz.dylib`, or calls the C library and the maths
library every program links through `ctypes.CDLL(None)`, which works on
Linux too; see [C interoperability](../extending/c-interop.md).

## Building libraries

`typy build --lib shared tools.py` writes `libtools.dylib`, whose install
name is its file name, so a program linked with it finds it through
`DYLD_LIBRARY_PATH` or where it is installed. With a release `5.1`, the
library is `libtools.5.dylib`, with the link `libtools.dylib`, and records
the compatibility version 5.0 and the current version 5.1. `--lib static`
writes `libtools.a`. [Naming a library by its version](../extending/c-interop.md#naming-a-library-by-its-version)
shows both systems' names.
