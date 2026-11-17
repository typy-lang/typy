# ADR-0049: Lay out a ctypes structure as C and pass it as System V does

## Status

Accepted

## Date

2026-11-17

## Context

FFI-03 accepts `ctypes.Structure` subclasses with a literal `_fields_`
list. ADR-0048 holds `ctypes` objects as counted objects over a C buffer,
which a structure fits: its buffer holds its fields. Three questions are
left: how a class statement becomes a class TyPy knows when it compiles,
where each field lies, and how a structure passes to a C function by value,
which `libffi` does for CPython and TyPy has to do itself.

CPython 3.12 to 3.15, measured on 2026-11-17, behave alike:

| Operation | CPython |
| --- | --- |
| `class P(ctypes.Structure): _fields_ = [...]` | Fields at C's offsets: `{int, double}` is 16 bytes, `{char *, int8, int64, bool}` 32 |
| `P(1, y=2.5)` | Positional fields in order, then keywords; too many is `TypeError: too many initializers`, a field given twice `TypeError: duplicate values` |
| `p.x`, `p.x = v` | A scalar field's value, converted and wrapped as `value` is |
| `p.inner`, `p.cells`, `p.next` | A new object over the field's memory, for a structure, an array or a pointer field |
| `p.inner = q` | Copies `q`'s bytes |
| `repr(p)`, `bool(p)`, `==` | `<module.P object at 0x...>`, true, identity |
| `restype = P`, `argtypes = [P]` | The structure by value: `div` returns `{int, int}` in one register, `lldiv` `{long, long}` in two |

## Options considered

1. **A program class.** Check the class statement as any class, with
   fields as attributes. Its instances would be TyPy objects, which a C
   function cannot read, and `p.inner` could not share `p`'s memory.
2. **A `ctypes` class read when the module is.** The declarations
   pre-pass of ADR-0047 reads the class statement, lays out its fields as
   C does, and replaces the statement with the binding of its name to the
   class. Its objects are ADR-0048's objects.

For passing by value:

1. **Through `libffi`** at run time, as CPython does. It adds a library
   and a call through it for each C call, which ADR-0047 avoids.
2. **As clang lowers a C struct** for the System V calling convention of
   x86-64 Linux, decided when the program compiles.

## Decision

TyPy takes the second option of each.

- **Classes.** A class statement at module level whose one base is
  `ctypes.Structure` and whose body holds a docstring, `pass` and one
  `_fields_` list of `(name, type)` pairs is a structure class. A field's
  type is a scalar type, `POINTER(T)`, `T * n` with `n` an integer
  literal, an earlier structure of the module or of a module it imports,
  or a name bound to one of these. Each field lies at the next offset its
  alignment allows, the alignment of a scalar being its size, and the size
  rounds up to the largest alignment. The statement becomes `P = <class>`,
  so `P` is a module variable as any class value of ADR-0048 is, and a
  module that imports `P` reads it through the variable.
- **Objects.** `P(...)` takes positional fields in order and keywords by
  name; too many, an unknown name or a field given twice is E0404 when the
  program is compiled, where CPython raises `TypeError` at run time. A
  scalar field reads and writes as `value` does; a structure, array or
  pointer field reads as a new object over its memory, which keeps the
  structure alive. Assigning a structure or an array copies its bytes and
  keeps the source alive with the memory, and assigning a pointer field
  writes its address and keeps the pointer. A pointer to a structure, an
  array of structures, and `byref` of one work as ADR-0048's for scalars;
  an item of an array of structures reads as an object over it.
- **By value.** A structure of no bytes passes nothing. One of up to 16
  bytes passes in one or two registers: each eight bytes in a vector
  register when every value in them is a `float` or a `double`, in a
  general register otherwise, each read without reading past the
  structure. When the registers left, six general and eight vector, cannot
  hold all of it, or it is larger, it passes in memory as a `byval` copy.
  A result in registers is stored into a new object of the class; a larger
  result is written by the function into a new object's buffer, passed as
  the `sret` parameter.
- **Not yet.** Methods and other class attributes, `_fields_` assigned
  after the class statement, bit fields, `_pack_` and `_anonymous_`, a
  subclass of a structure, and `ctypes.Union` are planned for 1.0
  (FFI-11).

## Consequences

- A C function reads and writes a structure the program holds, by pointer
  or by value, with no library between them, as fast as C calls C.
- The calling convention is x86-64 Linux's only; another target needs its
  own rules when TyPy targets it (CLI-08).
- A self-referring structure, such as a list node with a pointer to its
  own class, needs `_fields_` after the class statement, which waits for
  1.0.
- A structure whose pointer field points to itself keeps itself alive,
  since TyPy counts references and has no cycle collector; CPython's
  collector frees it.

## Related requirements

FFI-02, FFI-03, FFI-11, ADR-0047, ADR-0048.
