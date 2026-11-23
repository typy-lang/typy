# ADR-0108: Hold a bytearray in a growable bytes buffer it owns

## Status

Accepted

## Date

2026-11-23

## Context

LIB-28 asks for `bytearray` as CPython gives it: a mutable sequence of
bytes with every method of `bytes`, the list methods that change it
(`append`, `extend`, `insert`, `pop`, `remove`, `reverse`, `clear` and
`copy`), item and slice assignment and deletion, `+=` and `*=`, and
comparison with `bytes`.

A `bytes` is a `str` object of one byte per code point under its own type
ID, so it shares the string algorithms (DAT-06). A `str` object holds its
code points after its fixed part and cannot grow, and it caches its hash.
The methods of `bytes` are those string algorithms with results retyped as
`bytes`.

## Options considered

1. **A bytearray object that owns a `bytes` buffer with spare capacity.**
   The buffer's `length` is the bytearray's length, and its allocation holds
   `capacity` bytes and a final zero. Every algorithm of `bytes` that reads
   runs on the buffer as it is, a method that makes a new sequence makes a
   `bytes` and the result becomes the buffer of a new bytearray, and a
   change writes the buffer in place, reallocating it to grow.
2. **A bytearray object with a plain byte array**, copied into a new
   `bytes` for each method of `bytes`. Simple, but every `find` or
   `startswith` would copy the whole sequence.
3. **A list of `int` items.** Lists have the mutation already, but each
   byte would take eight bytes, and none of the algorithms of `bytes` would
   apply.

## Decision

Option 1. `BytearrayObject` is `#[repr(C)]`: the header with
`BYTEARRAY_TYPE_ID`, the address of its buffer, and the buffer's capacity.
The buffer has the type ID of `bytes`, so the functions of `bytes` read it
unchanged; it is never hashed and never given to the program, so its hash
cache is never stale. A result a function of `bytes` returns becomes the
buffer of the new bytearray without a copy, unless it is the buffer itself,
as for a `strip` that removes nothing, which is copied, so two bytearrays
never share a buffer. A bytes-like argument, `bytes` or `bytearray`, is
read through its buffer. A bytearray is unhashable, so it cannot be a key
of a dict or a member of a set, which the checker refuses at compile time.

## Consequences

- Every method of `bytes` is a method of `bytearray` with no copy of the
  receiver, and the new methods are the list methods and the assignments.
- Growing doubles the capacity, so `append` takes constant time on
  average, as CPython's does.
- `bytes(b)` copies the buffer, since the bytearray may change after it.
- The runtime has one more object type and type ID, and the checker one
  more type, `bytearray`, accepted wherever a bytes-like argument is.

## Related requirements

LIB-28, DAT-06, DAT-15, MEM-02.
