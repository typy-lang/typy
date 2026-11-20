# Getting started

This page installs TyPy from a release archive, then checks, builds and
runs a first program.

## What you need

TyPy runs on x86_64 and 64-bit Arm (AArch64) Linux with glibc 2.39 or
later, such as Ubuntu 24.04 or Debian 13, and on macOS 11 or later on
Apple silicon. It links programs with the system C toolchain, so `cc` must
be installed:

```sh
sudo apt install build-essential
```

On a Mac, Xcode's Command Line Tools provide it:

```sh
xcode-select --install
```

## Install from a release archive

Download the archive for your machine and its `.sha256` file from the
[release page](https://github.com/typy-lang/typy/releases):
`typy-0.9.0-x86_64-unknown-linux-gnu.tar.gz` on x86_64 or
`typy-0.9.0-aarch64-unknown-linux-gnu.tar.gz` on AArch64 Linux, as
`uname -m` names the machine. Check the archive and unpack it:

```sh
archive="typy-0.9.0-$(uname -m)-unknown-linux-gnu"
sha256sum --check "$archive.tar.gz.sha256"
tar -xzf "$archive.tar.gz"
export PATH="$PWD/$archive/bin:$PATH"
typy --version
```

On a Mac the archive is `typy-0.9.0-aarch64-apple-darwin.tar.gz`, which
[Using TyPy on macOS](../using/macos.md) installs, clearing the quarantine
a browser download carries.

The archive holds the `typy` command in `bin/` and the runtime library of
each target it builds for in `lib/typy/`: either Linux archive builds for
x86_64 and AArch64 Linux, and the Mac archive for macOS; see
[Building for another target][cross]. Keep the two together: `typy` finds
the runtime beside itself.

[cross]: ../using/command-line.md#building-for-another-target

## A first program

Save this program as `words.py`. Annotations are required on function
parameters and returns; a variable takes the type of its first value.

<!-- typy-expect: accepted -->
```python
def longest(words: list[str]) -> str:
    best = ""
    for word in words:
        if len(word) > len(best):
            best = word
    return best


counts: dict[str, int] = {}
text = "the quick brown fox jumps over the lazy dog the end"
for word in text.split():
    counts[word] = counts.get(word, 0) + 1
print(longest(text.split()), counts["the"], f"{len(counts)} distinct words")
```

## Check, build and run

`typy check` reports every problem without generating code:

```sh
typy check words.py
```

`typy run` builds the program and runs it. Arguments after `--` go to the
program:

```sh
typy run words.py
```

`typy build` writes a native executable beside the source, here `words`,
that runs without TyPy or Python installed:

```sh
typy build words.py
./words
```

`typy build --release` optimises for speed, and `--release-small` for
size; `-o PATH` names the executable. `typy build --emit=STAGE` writes
one stage of the compiler instead, such as `typed-ast` or `llvm-ir`.

## When a program is refused

TyPy refuses a program it cannot compile with CPython's meaning. Saved as
`mixed.py`, this list would need to hold both an `int` and a `str`, which a
`list[T]` cannot:

<!-- typy-expect: rejected E0409 at 1:13 -->
```python
mixed = [1, "two"]
print(mixed)
```

The diagnostic gives a code, the place, one sentence that says what is
wrong and the rule it falls outside:

```text
error[E0409]: the items of a list have one type; this is a str, and the list holds int
 --> mixed.py:1:13
```

[Troubleshooting](../diagnostics/troubleshooting.md) explains each code,
and the [language reference](../reference/index.md) lists what each
release accepts.
