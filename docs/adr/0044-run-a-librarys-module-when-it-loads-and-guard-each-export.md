# ADR-0044: Run a library's module when it loads and guard each export

## Status

Accepted

## Date

2026-11-17

## Context

ADR-0032 decides what a library built with `typy build --lib` exports: the
functions its module's `__all__` lists, with C types from their
annotations. It leaves open how such a library runs inside a program that
TyPy did not build, which CLI-09, FFI-05 and EXC-05 need for 0.5.

An executable starts in the runtime's `main`, which records the stack
limit the recursion check uses, keeps the arguments for `sys.argv`, runs
the module bodies through `__typy_main`, reports an exception nothing
caught, and flushes standard output and closes files at the end. A library
has no `main` of its own, and the program that loads it may be in C, C++,
Rust or Python. Experiments with the 0.5 runtime archive show three things
that decide the design:

- The runtime archive is one object, built with LTO, that defines `main`.
  A partial link of the program's object with it, with every global symbol
  but the exports made local, gives a static library that a C program with
  its own `main` links. A shared library linked with a version script that
  exports only the exports loads into CPython through `ctypes`.
- Without the runtime's `main`, nothing flushes the runtime's buffer of
  standard output: a `print` in the library's module never appears.
- The recursion check compares the stack pointer with a limit recorded when
  `main` starts. A call from another thread runs on another stack, where
  the check would fire at once or never.

## Options considered

For when the module bodies run:

1. **When the library loads**, from a constructor in `.init_array`: when
   the program starts for a library it links, and in `dlopen` for one it
   loads later, as `ctypes.CDLL` does. This is when CPython runs a
   module's body: when it is imported, before any of its functions is
   called.
2. **At the first call of an export**, behind a test on every call. A
   library whose exports are never called never runs its body, and the
   body's output lands in the middle of the program's.
3. **In an initialising function the host calls**, as many C libraries
   ask. A host that forgets it calls functions whose module variables have
   no values.

For the runtime's symbols in a static library:

1. **Leave them global.** A program that links two TyPy libraries, or a
   TyPy library into a TyPy program, gets every runtime symbol twice.
2. **Make every symbol but the exports local**, after a partial link of
   the program object and the runtime archive into one object. Each
   library keeps a runtime of its own, as each shared library does.

## Decision

TyPy uses option 1 for when the module bodies run, and option 2 for the
symbols.

- **The module.** The main file of a library build is a module named
  after its file, so `mathlib.py` is `mathlib`, its `__name__`, and
  `if __name__ == "__main__":` code does not run, as on an import.
  `sys.argv` is `['']`, as in an embedded CPython.
- **Loading.** The object of a library has a constructor that records the
  stack limit, runs the module bodies and flushes standard output. An
  exception the bodies raise is handled as one that leaves an export.
  A destructor flushes standard output and closes the files the program
  opened, as `main` does when it ends.
- **Each export.** The C symbol is the function's name, and calls a
  wrapper that records the stack limit from the calling thread's stack
  pointer, converts each argument from its C type, calls the function,
  converts the result, and flushes standard output, so the library's
  output and the host's keep their order. `int`, `float` and `bool` are
  `int64_t`, `double` and `bool`, and a result of type `None` is `void`;
  an `int` result outside `int64_t` raises `OverflowError`.
- **An exception that leaves an export** never reaches C (EXC-05): a
  `SystemExit` ends the process with its status, as when it leaves a
  program's module, and any other exception is printed as an uncaught
  exception is, and the process aborts.
- **Names.** `__all__` is a list or tuple of string literals bound once at
  module level, and each name is a function of the module with positional
  parameters, no type parameters, and parameters and a result of the types
  above. An export may not be named `main`, start with `typy_`, `__typy`
  or `_TY`, or be a C library function the runtime calls, such as `read`
  or `free`, since inside the library its definition would take that
  function's place. Each problem is E0428, which names the export.
- **Threads.** The runtime's state is not shared safely between threads,
  so a host calls a library's exports from one thread at a time.
- **Linking.** A static library is the program object and the runtime
  archive linked into one object with `cc -r`, its global symbols but the
  exports made local with `objcopy --keep-global-symbols`, in an archive
  made with `ar`. A shared library is linked with `cc -shared` and a
  version script that exports only the exports. The runtime's `main` stays
  in the archive and is local or unexported in both. The output is
  `libNAME.a` or `libNAME.so` beside the main file unless `-o` names
  another path, and the C header `NAME.h` is written beside it.

## Consequences

- A library behaves as an imported module: its body runs once, before any
  export is called, and a C program, a Python program through `ctypes`
  and a TyPy program see the same output in the same order.
- Each library carries the runtime, about 1 MiB in a static library before
  the host's linker removes what no export reaches.
- Every call of an export pays for recording the stack limit and testing
  the output buffer, a few instructions beside the call.
- A static library needs `objcopy` and `ar` besides the C compiler, which
  the binutils of every Linux C toolchain provide (A-01).
- The recursion check of a call from a thread assumes the thread has the
  stack size `getrlimit` gives below the call, which is glibc's default
  for a new thread.

## Related requirements

CLI-09, FFI-05, EXC-05, FUN-02, STD-05, LIB-04, A-01, ADR-0032, ADR-0043.
