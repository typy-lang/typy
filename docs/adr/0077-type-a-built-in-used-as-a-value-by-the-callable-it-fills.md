# ADR-0077: Type a built-in used as a value by the `Callable` it fills

## Status

Accepted

## Date

2026-11-20

## Context

`sorted(words, key=len)`, `max(rows, key=len)`, `map(int, line.split())`
and `sorted(numbers, key=str)` pass a built-in function or a built-in type
where a function is expected (FUN-08, FUN-14). Up to 0.9 TyPy refuses
them: `len` as a value is E0201, `str.upper` and `xs.append` are E0412,
and `key=str` is E0404.

Most built-ins have no single signature. `len` takes any sized value,
`abs` an `int` or a `float`, `str` any value, and TyPy checks each call of
a built-in by its own rules, not through a signature table. Methods of
built-in types have a table of names and kinds (LIB-03), but several of
their parameters accept more than one type, such as `str.split`'s `sep`,
a `str` or `None`.

CPython 3.12.16 and 3.15.2, checked on 2026-11-20, write `len` as
`<built-in function len>`, `math.sqrt` as `<built-in function sqrt>`,
`str.upper` as `<method 'upper' of 'str' objects>`, `xs.append` as
`<built-in method append of list object at 0x...>` and `str` as
`<class 'str'>`. `callable(len)` and `callable(str)` are true, and
`callable(3)` and `callable(None)` are false.

## Options considered

1. **Take the type from the `Callable` the value fills.** The checker
   checks a call of the built-in with arguments of the `Callable`'s
   parameter types, by the rules it already has, and the call's result
   must pass unchanged as the `Callable`'s result (NFR-19).
2. **Give every built-in fixed signatures,** written as protocols such as
   `Sized`. Those protocols need generic protocols over built-in types,
   planned for 0.13 (GEN-13, GEN-17).
3. **Accept only built-ins of one signature,** such as `chr` and `ord`,
   and refuse `len`, `str` and the other common keys.

## Decision

Option 1, with a fixed type for a built-in of one signature.

- **Expected type.** A built-in function, a function of a module TyPy
  provides such as `math.sqrt`, or a built-in type whose call TyPy types,
  such as `int`, `str` or `list`, read as a value where a
  `Callable[[A1, ..., An], R]` is expected, is checked as the call
  `name(a1, ..., an)` with arguments of types `A1` to `An`. The call must
  be accepted, and its result must pass unchanged as `R`; otherwise the
  value is E0404 with the call's own message.
- **Nothing expected.** A built-in whose parameters each take one type,
  with no optional parameter, such as `chr` and `ord`, has that type, so
  `f = chr` is `Callable[[int], str]`. Any other built-in read where no
  `Callable` is expected, such as `f = len`, is E0203, with the help to
  annotate the variable.
- **The value.** For each built-in and `Callable` type, the compiler makes
  a hidden function whose body is the call, and its immortal object. The
  descriptor's identity key names the built-in (ADR-0076), so `repr`
  writes `<built-in function len>`, or `<class 'str'>` for a type, and
  `is` and `==` hold between two reads.
- **Methods of built-in types.** `xs.append` and `"-".join` are bound
  objects that capture the receiver and compare it by identity, as bound
  methods of the program do (ADR-0060). `str.upper` read through the type
  is an immortal object that takes the receiver as its first argument.
  Their types come from the method table, and a parameter that accepts
  several types takes the one the expected `Callable` gives.
- **`callable(x)`** is a constant for every type TyPy has, true for a
  `Callable`, a type object and a built-in function, false otherwise,
  and is tested when the program runs only for a union or an optional
  that holds a `Callable`, which it narrows as `isinstance` does. `x` is
  still evaluated (LIB-23).
- **Not in this record.** A class of the program or an exception class as
  a `Callable` is planned for 0.14 (FUN-16). `property`, `staticmethod`
  and `classmethod` objects as values, including a property read through
  its class such as `Box.v`, are planned for 1.0 (FUN-17); until then they
  are E0201, which also stops `Box.v(box)` from calling the getter.

## Consequences

- The keys and functions Python programs pass most often, `len`, `abs`,
  `str`, `int`, `str.lower` and `xs.append`, work as in CPython, with
  CPython's identity and text.
- A built-in used with several `Callable` types makes one hidden function
  for each, as a generic function makes one copy for each set of types
  (ADR-0023).
- `f = len` without an annotation is refused where CPython accepts it; a
  type checker infers an overloaded type there that TyPy cannot hold.

## Related requirements

FUN-08, FUN-14, FUN-16, FUN-17, LIB-03, LIB-23, NFR-19, ADR-0023,
ADR-0060, ADR-0076.
