# TyPy research: the command lines of compilers

| Field | Value |
| --- | --- |
| Document | TyPy research: the command lines of compilers |
| Version | 1.0 |
| Status | Approved |
| Owner | typy-lang contributors |
| Last updated | 2026-11-23 |

## 1. Purpose

This document compares the command line of `typy` with those of the
compilers and the interpreter a TyPy user also works with: `rustc`,
`clang`, `cargo`, CPython, and the compilers of Python Codon, Nuitka, mypyc
and Cython. For each area of work, such as choosing how hard to optimise,
inspecting what a stage produced or controlling diagnostics, it records
what the other tools offer, what `typy` offers, and what TyPy needs, and it
proposes questions for the requirements.

It builds on the software requirements specification, version 1.123, whose
CLI requirements define today's `typy`, and on two of its rules: `typy`
reads no environment variable that changes its output except `TYPY_HOME`
(CLI-11), and a build targets the baseline CPU of its triple, so it runs on
every machine of the target and two builds of one source are the same
bytes (NFR-06).

## 2. Method

Two experiments ran in the development container on 2026-11-22 and
2026-11-23:

- **X9** collects the help of each tool: `rustc --help -v`, `rustc -C
  help` and the values of `rustc --print`; `clang --help`; `cargo build
  --help` and `cargo --list`; `python3.15 --help-all`; `codon --help` and
  `codon build --help`; `python -m nuitka --help`; `mypyc --help`; and
  `cython --help`. LLVM's `opt` and `llc` are not installed in the image;
  `clang` and `rustc` stand for them, as they pass options on to LLVM.
- **X10** measures what linking the runtime into each program as LLVM
  bitcode, so the optimiser inlines it, would give, the strongest form of
  link-time optimisation `typy` could offer: section 4.2.

| Tool | Version |
| --- | --- |
| `rustc`, `cargo` | 1.99.1 |
| `clang` | Debian 19.1.7 |
| CPython | 3.15.2 |
| Codon | 0.20.6 |
| Nuitka | 4.3.5 |
| mypyc | mypy 2.4.1 |
| Cython | 3.3.1 |
| `typy` | 0.11.0 |

## 3. `typy` today

`typy` has four commands. `check FILE` reports every diagnostic, `build
FILE` builds an executable or a library or writes one stage, `run FILE --
ARGS` builds into a cache and runs, and `explain CODE` explains a
diagnostic. Across them it takes:

| Option | Commands | What it does |
| --- | --- | --- |
| `--release`, `--release-small` | `build`, `run` | Optimise for speed, or for size |
| `--emit STAGE` | `build` | Write one stage: `ast`, `typed-ast`, `tyir`, `tyir-mem`, `llvm-ir`, `asm` or `obj` |
| `-o PATH` | `build` | Where the output goes |
| `--target TRIPLE` | `check`, `build` | Build for another supported target |
| `--lib static\|shared` | `check`, `build` | Build a C library |
| `--python-floor VERSION` | `check`, `build`, `run` | The oldest CPython the program supports |
| `--error-format text\|json` | all | How diagnostics are written |
| `--deny-warnings` | all | Fail on any warning |

## 4. What the tools offer

### 4.1 How hard to optimise, and for which processor

| Need | Offered by | `typy` |
| --- | --- | --- |
| Optimisation levels | `rustc -C opt-level=0..3\|s\|z` and `-O`; `clang -O0` to `-O3`, `-Os`, `-Oz`; Codon `--release` and `--debug` | Two modes beside the debug build |
| A processor's own instructions | `rustc -C target-cpu=NAME\|native`, `-C target-feature`; `clang -march`, `-mcpu`, `-mtune`; Codon `--march`, `--mcpu` | None: always the baseline CPU |
| Link-time optimisation | `rustc -C lto`, `-C linker-plugin-lto`; `clang -flto`, `-flto=thin` | None |
| Profile-guided optimisation | `rustc -C profile-generate`, `-C profile-use`; `clang -fprofile-instr-generate`, `-fprofile-use`; Nuitka's PGO options | None |
| Options for LLVM itself | `rustc -C llvm-args`, `-C passes`, `-C no-vectorize-loops`; Codon passes every LLVM option | None |

A processor's own instructions break NFR-06's promise that a build runs on
every machine of its target, so `typy` can offer them only as a choice the
user makes, never as a default, and `native` makes a build depend on the
machine that made it.

### 4.2 Linking the runtime as bitcode (X10)

X10 compiled the runtime with the `runtime` profile to one LLVM bitcode
file, 2.7 MB; `rustc` 1.99.1 uses LLVM 23.1.1 and `typy` LLVM 23.1.6, so
`typy`'s LLVM reads it. A tool built on `typy`'s own `inkwell` linked each
benchmark program's unoptimised IR with the bitcode, made the runtime's
definitions, `main` apart, internal, and ran the `O2` pipeline over the
whole. Every program printed CPython's output.

| Measure | Release build | Runtime linked as bitcode |
| --- | --- | --- |
| Geometric mean run time against C (gcc -O2) | 5.29 | 4.96 |
| Largest gains | | recursion 1.29, spectralnorm 1.25, binarytrees and strings 1.16 |
| Losses | | nbody 0.89, fasta 0.96 |
| Build time of one program | 0.06 to 0.17 s | 1.95 to 3.70 s |

Reading and linking the bitcode took 0.2 s; optimising and generating code
for the 187 runtime functions a program reached took the rest, on every
build, where the archive compiles them once. Inlining the runtime is worth
6 percent at twenty times the build time: a choice for a user who wants the
last few percent, not a default.

### 4.3 Inspecting what the compiler made

| Need | Offered by | `typy` |
| --- | --- | --- |
| Write a stage | `rustc --emit asm,llvm-bc,llvm-ir,obj,link,dep-info` with several at once; `clang -S`, `-emit-llvm`, `-c`, `-E`; Codon `--llvm`, `--obj`; Cython writes C | `--emit`, one stage at a time; no IR before optimisation, no bitcode |
| Keep the intermediate files | `rustc -C save-temps`; `clang -save-temps` | None |
| Show the commands run | `clang -###`; `rustc -v` and `--print link-args`; `cargo -v` and `-vv` | None |
| Ask the compiler about itself | `rustc --print` of 18 kinds, such as `target-list`, `target-cpus`, `target-features`, `sysroot`, `link-args` and `cfg`; `clang -print-targets` and `-mcpu=help` | `--version` only |
| Time each stage | `clang -ftime-report`, `-ftime-trace`; `cargo build --timings`; Nuitka's `--report` | None |
| Why an optimisation did or did not happen | `clang -Rpass`, `-Rpass-missed`, `-Rpass-analysis`; `rustc -C remark` | None |
| Debug information | `rustc -g`, `-C debuginfo=0..2`, `-C strip`, `-C split-debuginfo`; `clang -g` | None: release builds strip it |

### 4.4 Diagnostics

| Need | Offered by | `typy` |
| --- | --- | --- |
| Per-diagnostic levels | `rustc -A`, `-W`, `-D`, `-F`, `--cap-lints`; `clang -W...`, `-Werror=...`; CPython `-W action:message:category` | `--deny-warnings`, all or nothing |
| Format | `rustc --error-format human\|json\|short`, `--json`; `clang -fdiagnostics-format`; `cargo --message-format` | `--error-format text\|json` |
| Colour | `rustc --color`, `cargo --color`, `clang -fcolor-diagnostics`, Codon `--color` | None |

### 4.5 Running a program

| Need | Offered by | `typy` |
| --- | --- | --- |
| Run code given on the command line | CPython `-c CODE` | None |
| Run a module by its name | CPython `-m MODULE` | None |
| Developer checks | CPython `-X dev`, `-X faulthandler`, `-W error` | None |
| Measure imports | CPython `-X importtime` | None |
| Memory and leak checks | `rustc -Z sanitizer`, `clang -fsanitize=address,undefined`; CPython `-X tracemalloc`, `-X showrefcount` | None for users; the conformance suite builds with AddressSanitizer |
| Profilers | CPython `-X perf`, `-X perf_jit` | None: no frame pointers or debug information to profile by |

### 4.6 The work around a build

`cargo` adds the commands a project needs beside building: `check`,
`build`, `run`, `test`, `bench`, `doc`, `clean`, `fmt` and `clippy` through
its tools, and `--timings`, `-j` and `--message-format` for the whole run.
mypyc and Nuitka take their project options from configuration files and
the command line alike. `typy` reads `typy.toml` and has no commands for
tests, benchmarks, formatting or its cache.

## 5. Findings

1. **Optimisation and processor.** `typy` offers two of the five common
   optimisation levels and no processor choice, link-time optimisation,
   profile-guided optimisation or LLVM options. The levels and a target
   CPU are small to add, since `typy-codegen` already chooses an LLVM
   pipeline and a CPU; a processor choice must stay opt-in under NFR-06.
2. **Inspection.** The largest gap: no timings, no shown commands, no kept
   temporary files, no IR before optimisation, no remarks, no debug
   information and no way to ask `typy` about its targets and paths. All but
   debug information and remarks are small; remarks need debug information
   to point at Python lines.
3. **Debug information** is the foundation of several others: debuggers,
   profilers such as `perf`, and remarks. It needs line tables from TyPy's
   spans through TyIR to LLVM's debug metadata.
4. **Diagnostics.** Per-code levels and colour are small, and match what
   `rustc` and CPython's `-W` let a user do.
5. **Running.** `-c` and `-m` are small and make `typy` feel like `python`;
   sanitizers are near, since the AddressSanitizer runtime is built
   already; profilers wait for debug information.
6. **Inlining the runtime** (X10) is a valid opt-in for the fastest
   builds, not a default.
7. Every option that changes the output must join the key of `typy run`'s
   cache, so a cached build is never one made with other options.

## 6. Questions for the requirements

The specification settles these as OI-45 to OI-48, version 1.124.

| Question | Options |
| --- | --- |
| How are options spelt? | Dedicated flags for each, as `clang` and CPython do; or a few flags plus `-C KEY=VALUE` for codegen, as `rustc` does, which scales without crowding `--help` |
| Which come in 0.12, and which later? | The small ones first: levels, target CPU, timings, shown commands, kept files, a `print` command, colour, per-code levels, `-c` and `-m`; then debug information, sanitizers, remarks and profiling |
| Are `native` and runtime inlining offered? | Both are opt-in choices that trade NFR-06's portability or build time for speed |
| Does `typy` gain project commands? | `test`, `bench`, `fmt` and `clean`, as `cargo` has, or none until projects need them |
