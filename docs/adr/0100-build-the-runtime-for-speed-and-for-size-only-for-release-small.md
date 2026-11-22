# ADR-0100: Build the runtime for speed, and for size only for `--release-small`

## Status

Accepted

## Date

2026-11-22

## Context

The runtime archive every program links was built with Rust's
`opt-level = "z"`, the smallest code, so that a hello world built with
`--release-small` keeps its runtime under NFR-02's budgets: 48 KB in all
and 32 KB without the large-integer and cycle-collection parts.

NFR-21 asks that no program of the benchmark suite built with `--release`
run slower than CPython 3.15. On 2026-11-22, after the byte paths for
strings, five of the twelve still did, the strings and dicts most of all.
Their time goes to runtime functions, such as hashing, dict lookups and
string methods, which `"z"` compiles without inlining or unrolling. The
same programs against the same runtime source, times against CPython's:

| Runtime `opt-level` | knucleotide | dicts | strings | parser | fannkuchredux | Core of a hello world |
| --- | --- | --- | --- | --- | --- | --- |
| `"z"` | 2.06 | 1.29 | 1.15 | 1.00 | 1.04 | 17.3 KB |
| `"s"` | 1.29 | 0.94 | 0.86 | 0.70 | 0.68 | 19.4 KB |
| 2 | 1.02 | 0.73 | 0.74 | 0.65 | 0.68 | 29.0 KB |
| 3 | 0.98 | 0.69 | 0.70 | 0.64 | 0.65 | 30.7 KB |

## Options considered

1. **Two archives:** one for speed, which debug and `--release` builds
   link, and one for size, which `--release-small` builds link.
2. **One archive at `opt-level = 3`.** Every benchmark program beats
   CPython, but the core runtime takes 30.7 of NFR-02's 32 KB, which
   leaves no room for the runtime of later releases.
3. **One archive at `opt-level = 2`,** 29.0 KB, with knucleotide still at
   CPython's time.

## Decision

Option 1, chosen by the project on 2026-11-22. The `runtime` profile
builds with `opt-level = 3` into `<prefix>/lib/typy/<target>/`, the
archive debug and `--release` builds link. A new `runtime-small` profile
builds with `opt-level = "z"` into `<prefix>/lib/typy-small/<target>/`,
the archive `--release-small` builds link and NFR-02 measures.
`BuildMode::runtime_directory` names the directory of each mode.

## Consequences

- Programs built with `--release` run their runtime functions at full
  speed, and NFR-02 keeps its budgets for `--release-small` builds.
- A release package carries two archives for each target, and `make
  runtime` builds each target twice.
- The AddressSanitizer archive of the conformance suite builds from the
  speed profile, as the programs it checks link.

## Related requirements

NFR-21, NFR-02, CLI-05, ADR-0009, ADR-0051, ADR-0055.
