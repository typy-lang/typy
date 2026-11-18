# ADR-0060: Make a function value an object of its code and its captures

## Status

Accepted

## Date

2026-11-18

## Context

Up to 0.7 a function is only called, directly by its symbol, and a
function that may raise returns `{ result, exception }` (detailed design
section 10.2). 0.8 makes functions values (FUN-05) and adds nested
functions and closures (FUN-04), `lambda` (EXP-14) and `nonlocal`
(SMT-09). A value of type `Callable[[A, ...], R]` is stored, passed,
returned and called, and a list or dict item, an attribute or a capture
holds one word.

CPython 3.12 to 3.15 give a function value these properties, checked on
2026-11-18: `f == g` and `hash(f)` go by identity, and `repr(f)` is
`<function outer.<locals>.inner at 0x...>`, or `<function <lambda> at
0x...>`. A bound method `a.m` is a new object at each access, so `a.m is
a.m` is false, but `a.m == a.m` is true, as is the equality of their
hashes, since a bound method compares its instance by identity and its
function; `repr(a.m)` is `<bound method C.m of ...>`.

## Options considered

1. **A pair of a code pointer and an environment pointer, held as two
   words.** Calling allocates nothing, but a list item, an attribute and a
   capture hold one word, and a pair has no identity of its own.
2. **An object: the header, a pointer to the function's immortal
   descriptor, and its captures laid out as a tuple.** One word wherever a
   value goes; a function that captures nothing is one immortal object, so
   using a module-level function as a value allocates nothing.
3. **Defunctionalize: number every function of each `Callable` type and
   call through a switch.** It needs the whole program at once, against the
   separate compilation the project aims for (ADR-0016).

## Decision

Option 2.

- **The object.** The header with the type ID of a function, then
  `function: ptr`, at the offset where an instance keeps its class, to the
  function's descriptor, then the captures as the descriptor's tuple lays
  them out, which the runtime releases when it frees the object, as it
  releases an instance's attributes (ADR-0012).
- **The descriptor.** An immortal object of the function: its name and
  qualified name, the address of its code, and the tuple descriptor of its
  captures. A function that captures nothing has one immortal object beside
  its descriptor.
- **Calling a value.** The code takes the function object first, then the
  arguments, and returns `{ result, exception }` whether or not the
  function may raise, since a `Callable` type does not say. A module-level
  function or a method used as a value gets a thunk of that form that calls
  it directly.
- **Calling by name.** A call of a function by its name in the scope that
  defines it stays a direct call, of its code with the object as the first
  argument for a nested function, so only a call through a value is
  indirect.
- **Bound methods.** `obj.method` used as a value is a function object that
  captures the instance, marked as a method, so `==` and `hash` compare the
  instance by identity and the function, and `repr` writes `<bound method
  Class.method of ...>`, as CPython does.
- **Identity.** `is`, `==` and `hash` go by identity for every other
  function object; `repr` writes CPython's form with the object's address.
- **Safety.** A code address comes only from a descriptor the compiler
  emits, and a call through a value passes the arguments the value's
  checked `Callable` type gives, so an indirect call always reaches code
  of that signature. The object owns a reference to each capture and
  releases it when it is freed, as an instance does.

A built-in function or a method of a built-in type used as a value, such
as `key=len`, comes in 0.9 (FUN-08).

## Consequences

- Every function value is one word, and passing a module-level function
  allocates nothing.
- A call through a value costs an indirect call and the exception check of
  the raising form; a call by name costs what it costs today.
- The `key` argument of `sorted`, `list.sort`, `min` and `max` takes any
  function value, called through its code.

## Related requirements

FUN-04, FUN-05, EXP-14, SMT-09, FUN-08, ADR-0012, ADR-0016, ADR-0061.
