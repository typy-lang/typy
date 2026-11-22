# ADR-0092: Infer a function's result and an empty collection's items

## Status

Accepted

## Date

2026-11-21

## Context

TYP-01 requires an annotation on every parameter and every result, and
TYP-02 gives an unannotated local the type of its first assignment. So a
function without `-> T` is E0204, and `items = []` followed by
`items.append(1)` is E0410, though CPython runs both and C++14 infers a
result from its `return` statements (OI-33).

Separate compilation reads each module's interface from its signatures
(ADR-0016), and a library's exports take their C types from their
annotations (FFI-05).

## Options considered

1. **Keep TYP-01 as it is.**
2. **Infer results and empty collections, and keep parameter
   annotations.**
3. **Also compile unannotated parameters as type parameters,** one copy
   for each set of argument types, as C++20 `auto` parameters and Codon
   do. An error inside such a function shows at the call that makes the
   copy, and an interface cannot be read from the signature.

## Decision

Option 2.

- **Results.** A function without a result annotation returns the type
  its `return` statements give: one type, their union, or `None` when it
  has no `return` with a value. A generator function without one is
  E0204, since its type depends on what it yields and is sent.
- **Where an annotation stays required.** A function that calls itself,
  directly or through others, a method that is overridden or overrides
  one, and a function a library exports keep the annotation, since their
  result must be known before their body is checked or is part of an
  interface; leaving it out is E0204 with that reason.
- **Empty collections.** An unannotated local assigned an empty list,
  dict or set display takes its item types from the first statement of
  the same function that adds to it, such as `append`, `insert`,
  `extend`, `add`, `update`, `setdefault` or an assignment to a
  subscript. Read before that statement, or never filled, it is E0410 as
  in 0.10.
- **Parameters** keep their annotations (TYP-01).

## Consequences

- Plain helper functions read as CPython programmers write them, while
  every interface between modules keeps its written types.
- The checker checks a function's body before the calls that use its
  result, in the order of the call graph, and reports a cycle through an
  unannotated function as E0204.
- An inferred result is shown in messages as the type it was inferred to.

## Related requirements

TYP-26, TYP-27, TYP-01, TYP-02, FFI-05, OI-33, ADR-0016.
