# ADR-0048: Hold a ctypes object as a counted object over a C buffer

## Status

Accepted

## Date

2026-11-17

## Context

ADR-0047 compiles `ctypes` declarations and the scalar types of FFI-03 into
direct C calls, whose values are TyPy values. The rest of FFI-03 is about
values that live in C's memory: an object such as `ctypes.c_int(5)`, whose
`value` a C function writes through `ctypes.byref`; a pointer from
`ctypes.pointer`, `ctypes.POINTER(T)` or a C result, read through
`contents` and an index; and an array such as `(ctypes.c_int * 3)(1, 2, 3)`
that a C function reads as a pointer to its first item. Structures follow
in a record of their own.

CPython 3.12 to 3.15, measured on 2026-11-17, behave alike:

| Operation | CPython |
| --- | --- |
| `c_int(5)`, `c_int()` | An object holding 5, or 0; `None` or `False` for `c_char_p`, `c_void_p` and `c_bool` |
| `x.value = v` | Wraps an `int` to the type's width; any other type is `TypeError` |
| `repr(x)` | `c_long(7)` for `c_int64(7)`, by the canonical class; a pointer and an array show an address |
| `bool(x)` | The value's truth; a pointer's is whether it is null; an array's whether it has items |
| `x == y`, `hash(x)` | Identity; unhashable |
| `pointer(x)` | A pointer object that keeps `x` alive; `p.contents` is a new object over `x`'s memory each time |
| `p[i]`, `p[i] = v` | The value at the `i`th item, unchecked; a null pointer is `ValueError: NULL pointer access` |
| `(T * n)(...)` | An array of `n` items, the rest zero; too many items is `IndexError: invalid index` |
| `a[i]` | Checks the index, `IndexError: invalid index`, and counts from the end when negative |
| `f(x)` for a `POINTER(T)` parameter | Passes `x`'s address, as `byref(x)` does; an array passes its first item's address |
| `f(x)` for a `c_void_p` parameter | Takes a pointer, an array or `byref(x)`, and refuses a scalar object |

## Options considered

1. **A TyPy value with a hidden box**: a `c_int` held as an `int` and boxed
   only when a C function takes its address. Two names that share a
   `c_int`, and `p.contents`, share its memory in CPython, which a value
   cannot.
2. **A counted object over a buffer.** Each `ctypes` object is an object of
   the runtime: its class, the address of its buffer, and the object that
   owns the buffer when it is not its own, which keeps that object alive as
   CPython's `_b_base_` does. A scalar or an array owns a buffer after its
   header; `p.contents` and an item share their owner's; a pointer holds an
   address and the object it points into.

## Decision

TyPy uses option 2.

- **Classes.** Each `ctypes` class a program uses is a constant that code
  generation emits once: CPython's name for it (`c_long` for `c_int64`,
  `LP_c_int` for `POINTER(c_int)`, `c_int_Array_3`), its size, and the
  class of its item or target. A class is a value that holds nothing when
  the program runs, since its type says which class it is: a program calls
  it, multiplies it into an array class, passes it to `POINTER` and
  `sizeof`, binds it to a name, and names it in a declaration.
- **Scalars.** `T(value)` and `T()` make an object of each scalar type of
  ADR-0047; `value` reads and assigns it with ADR-0047's conversions, and
  `x.value += 1` reads, combines and assigns it; `repr`, truth and identity
  are CPython's.
- **What memory refers to.** The object that owns a buffer keeps alive
  each object a value in it refers to, under the value's address, as
  CPython's `_objects` does: the `bytes` of a `c_char_p`, written directly,
  through an item or through a pointer, and a pointer's target. A write
  through `pointer(x)` keeps the `bytes` in `x`, so they live as long as
  `x` does.
- **Pointers.** `POINTER(T)` names a pointer class in a declaration, by
  itself or through a name bound to it at module level, and `POINTER(T)()`
  is a null pointer; `pointer(x)` and `POINTER(T)(x)` point to `x` and keep
  it alive. `contents`, an index and an assignment to an index read and
  write the target; a null pointer raises CPython's `ValueError`. A
  `POINTER(T)` argument takes a pointer to `T`, an object of `T` passed by
  reference, an array of `T`, `byref` of an object of `T`, or `None`; a
  `POINTER(T)` result is a new pointer object that keeps nothing alive.
- **Arrays.** `T * n` and `n * T`, with `n` an integer literal, make an
  array class; its objects have `len`, checked indexes, assignment to an
  item, iteration, `in` and `reversed`, each item read as its value.
- **`byref` and `sizeof`.** `byref(x)` is accepted as an argument of a C
  function, where it is `x`'s address. `sizeof` of a class is a constant,
  and of an object its class's size.
- **Not yet.** `c_char`, `c_wchar`, `c_wchar_p`, `create_string_buffer`,
  `cast`, `addressof`, pointers to and arrays of pointers and arrays,
  arrays whose length is not an integer literal, array slices, `byref`
  elsewhere, assigning a pointer's `contents`, and `ctypes` classes as
  values, in annotations, `isinstance` and `type` are planned for 1.0
  (FFI-10). Until then a function takes and returns the values of an
  object, not the object.

## Consequences

- A C function that writes through a pointer writes the object the program
  holds, as in CPython, and an object the program still holds never frees
  memory a pointer reads.
- Every object costs an allocation, as in CPython, and so does each read of
  an item, which reads through a short-lived object over the item; a C call
  that takes `byref(x)` costs no more than one that takes an `int`.
- A pointer from a C result reads memory TyPy does not own, as in CPython;
  reading past what C gave is the program's own fault in both.
- An array class is known when the program compiles, so its length is an
  integer literal; CPython takes any `int`.

## Related requirements

FFI-02, FFI-03, FFI-10, ADR-0047.
