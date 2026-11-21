# ADR-0074: Define a stable ABI for TyPy modules at 1.0

## Status

Accepted

## Date

2026-11-20

## Context

Every TyPy program links statically today, and a TyPy library offers only a
C interface, described by its own annotations (ADR-0032). ADR-0016 plans
separate compilation with static linking first and leaves dynamic linking
of TyPy modules to a later decision, OI-23, since it needs object layouts,
the exception state and the identity of type objects to stay the same
across shared libraries, and one runtime shared by all of them.

Without such an interface, two TyPy libraries in one process each carry a
runtime of their own, a TyPy object cannot pass from one to the other, and
every program repeats the code of the modules it shares with others.

## Options considered

1. **No TyPy ABI.** TyPy modules link statically, and the C interface of
   ADR-0032 stays the only boundary between shared libraries. Layouts stay
   free to change in every release, and an interface between TyPy modules
   would need a new decision.
2. **A stable ABI at 1.0.** From 1.0, object layouts, the exception state
   and the identity of type objects are fixed for every 1.x release, the
   runtime ships as one shared library as well as a static one, and TyPy
   modules link to each other as shared libraries.
3. **Decide after 1.0.** Keep OI-23 open until the layouts settle.

## Decision

TyPy takes option 2. From 1.0 it defines a stable ABI for its modules:

- **What is fixed.** The layout of each object header and of each built-in
  object, the layout a class's instances take from its attributes in
  order, the exception state a call returns, the symbols of type objects,
  which identify a class across shared libraries, and the names and
  signatures of the runtime's exported functions.
- **One runtime.** The runtime ships as `libtypy_runtime.so` on Linux and
  `libtypy_runtime.dylib` on macOS as well as the static archive. A program
  that links any TyPy module dynamically links the shared runtime, so the
  process has one runtime, one exception state per thread and one
  identity for each type object.
- **Versioning.** The ABI's version is the major release. Every 1.x release
  keeps it, so a module built by 1.x links with modules and a runtime of
  any later 1.x release; a change to anything fixed above waits for 2.0.
- **Static linking stays.** A program may still link everything
  statically, as today, and the C interface of ADR-0032 is unchanged.

## Consequences

- Layouts may change freely until 1.0. Before 1.0 the layouts are reviewed
  for what 1.x needs, such as room in the object header, since none can
  change after it.
- The interface files of ADR-0016 record the ABI version beside each
  interface hash, and linking refuses a module of another major release
  with a diagnostic, never a program that fails at run time.
- Each release from 1.0 is tested against modules built by the release
  before it.
- Inlining across a shared library boundary is lost; a release build that
  needs it links statically.

## Related requirements

MOD-16, ADR-0016, ADR-0032, OI-23.
