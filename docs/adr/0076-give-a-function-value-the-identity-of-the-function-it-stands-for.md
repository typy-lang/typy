# ADR-0076: Give a function value the identity of the function it stands for

## Status

Accepted

## Date

2026-11-20

## Context

0.8 made a function the program defines a value: an object whose header
points to the function's immortal descriptor, compared by identity, with
CPython's `repr` (ADR-0060). 0.10 adds values that stand for a function
without being its own object:

- A built-in function used as a value, such as `len` in
  `sorted(words, key=len)`, compiles to a hidden function for each
  `Callable` type it fills (FUN-08, ADR-0077), so `len` filling
  `Callable[[str], int]` and `len` filling `Callable[[list[int]], int]`
  are two objects.
- A function with defaults, `*args` or `**kwargs` used as a shorter
  `Callable` is called through an adapter (FUN-13, ADR-0080).
- A method read through a protocol value, or an override held otherwise,
  is bound through an adapter or a copy (FUN-09, ADR-0079).

CPython 3.12.16 and 3.15.2, checked on 2026-11-20, give these values the
identity of the function itself: `len is len` and `str.upper is
str.upper` are true, `repr(len)` is `<built-in function len>`, and a
function passed where a `Callable` is expected is the same object
(`<function total at 0x...>`). A bound method is a new object at each
access, so `xs.append is xs.append` is false while `==` is true, with
equal hashes.

## Options considered

1. **Identity of the object, as in 0.8.** `len is len` would be false
   when the two reads fill different `Callable` types, and `repr` of an
   adapter would name the adapter.
2. **One object per function, shared by every typed view,** with the
   types checked at each call. Every call through a value would pay a
   check, which ADR-0073 rejected for calls with unpacking.
3. **An identity key in the descriptor.** Each descriptor names the
   function it stands for: itself for a function of the program, the
   original for an adapter, and the built-in for a hidden function.
   `is`, `==`, `hash` and `repr` go by the key.

## Decision

Option 3.

- **The key.** A descriptor gains the address of the descriptor it stands
  for, which is its own address for a function of the program. The
  descriptor of an adapter or a hidden function points to the original's
  descriptor, or to one immortal descriptor per built-in function, such as
  `len`, that the runtime holds.
- **`is` between function values** that bind no receiver compares the
  keys, so `len is len` and `f is total` are true whichever `Callable`
  types they fill. `is` between bound methods compares the objects, as
  each access makes a new one, as in CPython.
- **`==` and `hash`** compare the key, and for a bound method also the
  receiver by identity, as ADR-0060 decides for methods of the program.
- **`repr`, `__name__` and `__qualname__`** come from the descriptor the
  key names: `<built-in function len>`, `<method 'upper' of 'str'
  objects>`, `<built-in method append of list object at 0x...>`,
  `<function total at 0x...>` or `<bound method Square.area of ...>`.
- **Calls** still go through the object's own code, so the key costs
  nothing when a value is called.

## Consequences

- A program sees CPython's identity, equality, hash and text for every
  function value, whatever hidden function or adapter the compiler made.
- The descriptor grows by one word, and a function value made by the
  program writes its own address there.
- `is` between two function values is a comparison of keys, not of
  objects, so the compiler may make as many hidden functions and adapters
  as the `Callable` types need without a program seeing more than one
  function.

## Related requirements

FUN-05, FUN-08, FUN-09, FUN-13, FUN-14, ADR-0060, ADR-0077, ADR-0079,
ADR-0080.
