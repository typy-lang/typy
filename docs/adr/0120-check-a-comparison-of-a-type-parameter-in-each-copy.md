# ADR-0120: Check a comparison of a type parameter in each copy

## Status

Accepted

## Date

2026-11-23

## Context

TyPy compiles a generic function once for each set of type arguments its
calls give (GEN-03), and checks each copy with those types. It also
checks the generic body once, with each type parameter an opaque type
that offers only what its bound offers. A comparison such as `x < a[mid]`
of an unbounded `T` is refused there with E0407, though every copy the
program makes may compare its values well.

Typed Python writes such a function with a type variable bound to a
protocol, typeshed's `SupportsRichComparisonT`, which the built-in types
satisfy. TyPy's protocols take instances of the program's classes only,
and `Self` in a protocol is planned for 0.13 (TYP-36), so the standard
library modules TyPy writes in Python (LIB-27), such as `bisect` and
`heapq`, cannot say that their items compare.

## Options considered

1. **Bound the type parameter by constraints,** such as
   `[T: (int, float, str, bytes)]`. A heap of `(priority, item)` tuples,
   the most common use of `heapq`, is refused.
2. **Let built-in types satisfy a comparison protocol** with `Self`. It
   needs TYP-36, planned for 0.13.
3. **Leave a comparison of a type parameter to the copies:** the check of
   the generic body accepts it, as a `bool`, and each copy checks it with
   its own types, as every other operation of a copy is checked.

## Decision

Option 3. While the generic body is checked, `comparison_error` reports
no comparison of which a side's type holds a type parameter; the copy
made for a call reports one its types do not allow, with E0407 at the
comparison.

## Consequences

- A generic function may compare its values with `<`, `<=`, `>`, `>=`,
  `==`, `!=`, through `sorted`, `min`, `max` and a sort key, as CPython
  runs it, and each copy is checked as concretely as any other code.
- mypy refuses such a function without a bound; TyPy accepts more, and a
  call that gives types that do not compare is refused where the copy
  compares them, rather than at the call.
- Other operations of an unbounded type parameter, whose result types
  would depend on the copy, stay refused.

## Related requirements

GEN-18, GEN-03, LIB-27, TYP-36, ADR-0119.
