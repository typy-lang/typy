# ADR-0019: Open text files over buffered descriptors with CPython's errors

## Status

Accepted

## Date

2026-11-12

## Context

LIB-04 asks for `open` for text files in 0.3, after `sys.stdout` and
`sys.stderr`, which are text files of the type `_io.TextIOWrapper`.
CPython builds a text file in three layers: a raw `FileIO` over the file
descriptor, a `BufferedReader` or `BufferedWriter`, and a
`TextIOWrapper` that decodes, encodes and translates newlines. Programs
see only the top layer, but its behaviour comes from all three:

- Reading translates `\r\n` and a lone `\r` to `\n` (universal
  newlines), also when the two bytes of `\r\n` fall in different reads.
- The text layer decodes 8192 bytes at a time (`_CHUNK_SIZE`), so a
  `readline` whose own line is valid raises `UnicodeDecodeError` for an
  invalid byte later in the same chunk. The error's position counts from
  the start of the chunk, and `read()` decodes the whole rest at once.
  CPython 3.12 and 3.15 agree on this, though 3.15 raised
  `io.DEFAULT_BUFFER_SIZE` from 8192 to 131072.
- Writes are buffered: another file object reads what was written out
  only after `flush`, `close` or the end of the program, when CPython
  closes every file still open.
- A failed open is an `OSError` subclass chosen by `errno`, written as
  `[Errno 2] No such file or directory: 'name'`, with the text of the C
  library's `strerror` and the `repr` of the file name.
- Reading a file opened for writing raises `io.UnsupportedOperation: not
  readable`; that class derives from both `OSError` and `ValueError`.
  Any operation on a closed file raises `ValueError: I/O operation on
  closed file.`
- The mode is checked when `open` runs: `"rw"` raises `ValueError: must
  have exactly one of create/read/write/append mode`.

The user chose text modes only for 0.3: `r`, `w`, `a` and `x`, each
with an optional `t`, in UTF-8.

## Options considered

1. **The C library's `FILE` streams.** `fopen`, `fgets` and `fputs` give
   buffering for free, but not CPython's chunked decoding, its error
   positions or its newline translation, and `strerror` texts differ
   between glibc and musl.
2. **A text file object in the runtime over a file descriptor.** The
   runtime reads 8192-byte chunks, validates each as CPython's
   incremental UTF-8 decoder does, translates newlines with a held-back
   `\r`, and buffers writes. Errors take a fixed table of glibc's
   `strerror` texts.
3. **`open` written in Python over `_typy` primitives (ADR-0007).** The
   chunking and decoding logic would be Python over byte primitives, which
   needs `bytearray` and slicing of `bytes` that 0.3 does not have.

## Decision

TyPy uses option 2.

- `open(file, mode, encoding=...)` returns a `TextIOWrapper`, the type of
  `sys.stdout`. `file` is a `str`. `mode` is a string literal, so the
  checker knows it is a text mode; a mode CPython refuses for every
  value is E0404 with CPython's message, a binary mode, a `+` mode or a
  mode that is not a literal is E0201, planned for 0.4 (LIB-14).
  `encoding` is `None` or a literal name of UTF-8; another encoding and
  the arguments `buffering`, `errors`, `newline`, `closefd` and `opener`
  are E0201 for 0.4.
- A file object holds its descriptor, its name, its mode as written, a
  read buffer of one chunk plus the bytes of a character cut off at its
  end, and a write buffer of 8192 bytes. Reading follows CPython's chunk
  rule above, with its error positions and bytes. `read(size)`,
  `readline(size)`, `readlines()`, `write(text)`, `writelines(lines)`,
  `flush()` and `close()` follow `TextIOWrapper`, and iterating over a
  file gives its lines, through the runtime's iterator objects
  (ADR-0014).
- `close()` of `sys.stdout` or `sys.stderr` writes it out and leaves it
  open, since `print` writes to standard output without a check for a
  closed file; closing them is planned for 0.4 (LIB-14).
- `with open(...) as f:` binds the file and closes it on every way out.
  Entering a closed file raises `ValueError`, as `__enter__` does.
- The runtime keeps a list of open files and closes each at the end of
  the program, so buffered writes reach the file as in CPython. A file
  object whose last reference goes is closed then.
- The runtime defines `OSError` and every subclass CPython has, and maps
  `errno` to the subclass as CPython does. The exception holds the
  arguments `(errno, strerror)` for its `repr` and the text with the file
  name for its `str`. `strerror` texts come from a table of glibc's, so
  both targets print the oracle's words. `IOError` and
  `EnvironmentError` name `OSError`, as in CPython.
- `io.UnsupportedOperation` derives from `OSError`, and the runtime's
  matching treats it as a `ValueError` too, which is its second base in
  CPython. A program cannot name it, since TyPy has no `io` module.
- A program builds an `OSError` with at most one argument. With two to
  five, CPython picks the subclass from `errno` and sets the attributes
  `errno`, `strerror` and `filename`; that is planned for 0.4 (EXC-14).

## Consequences

- Programs read and write text files with CPython's results, including
  its errors and messages, on both targets.
- Reading a file that another file object has written but not flushed
  shows only what was written out. CPython's write buffers differ in
  size between versions, so such a program has no single expected
  output; the user guide names it.
- The runtime grows by the text file code and the `strerror` table, which
  only a program that calls `open` links (NFR-01).
- `seek`, `tell`, `truncate`, the attributes `name`, `mode` and `closed`,
  binary files and other encodings come in 0.4 (LIB-14).

## Related requirements

LIB-04, LIB-14, EXC-02, EXC-14, CST-05, CST-09, NFR-01, OI-14, ADR-0007,
ADR-0013, ADR-0014, ADR-0018.
