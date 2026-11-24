# ADR-0106: Pass nothing to LLVM's global command line

## Status

Accepted

## Date

2026-11-23

## Context

ADR-0104 gives code generation options as `-C KEY=VALUE`, and CLI-30, as
first written, named `llvm-args` among the keys: a list of options passed
on to LLVM, as `rustc -C llvm-args` passes them. LLVM reads such options
through `LLVMParseCommandLineOptions`, which sets LLVM's global options for
the rest of the process. Two of TyPy's rules stand against it: every
compiler crate forbids `unsafe` code, and inkwell offers no safe call for
that function; and no state is process-wide, while these options would
reach every later build in the same process, as the conformance suite runs
many.

## Options considered

1. **Offer `-C llvm-args`** through an `unsafe` call, and accept that the
   options outlive the build that gave them.
2. **Offer the options LLVM takes per build instead:** the processor and
   its features go to the target machine each build makes, so
   `-C target-feature=+avx2,-sse4a` gives what `llvm-args` is most often
   used for, with nothing global.
3. **Offer neither.**

## Decision

Option 2. `typy` passes nothing to LLVM's global command line. `-C` takes
`opt-level`, `target-cpu` and `target-feature`, each a setting of the one
build; CLI-30 names `target-feature` in place of `llvm-args`. A later need
for another LLVM option is met by a key of its own that reaches LLVM per
build, never through the global command line.

## Consequences

- No build changes another, so the conformance suite and any program that
  embeds the driver build alike whatever ran before.
- The compiler crates keep `#![forbid(unsafe_code)]`.
- Options LLVM offers only on its command line, such as its debugging
  switches, are not reachable from `typy`.

## Related requirements

CLI-30, ADR-0104.
