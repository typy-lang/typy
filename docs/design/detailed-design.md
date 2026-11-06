# TyPy detailed design

| Field | Value |
| --- | --- |
| Document | TyPy detailed design |
| Version | 1.0 |
| Status | Approved |
| Owner | typy-lang contributors |
| Last updated | 2026-11-06 |

## 1. Purpose and scope

This document specifies the code of TyPy: the package layout, the layering
rules, and each type with its fields, behaviour and errors, under the names
the code uses. It builds on the approved
[software architecture](../architecture/architecture.md), the
[domain model](../modeling/domain-model.md) and the decision records
ADR-0001 to ADR-0007.

It covers release 0.1: every requirement of the specification whose release
is 0.1. Each later release extends this document in the same change as its
code. Where a type gains fields or variants in a later release, the text
says so with the release number.

## 2. Workspace layout

```text
Cargo.toml                    workspace, shared lint settings
Cargo.lock                    committed
rust-toolchain.toml           Rust 1.99.1
crates/
  typy-diagnostics/src/       span, position, diagnostic, code, rule,
                              registry, render
  typy-source/src/            project, settings, source_file, module,
                              syntax (TyPy's AST), parse (Ruff wrapper)
  typy-stdlib/
    src/lib.rs                embeds the modules below
    modules/builtins.py       print, len, abs, min, max, repr, ord, chr
    modules/_typy.pyi         declarations of the runtime primitives
  typy-check/src/             types, scope, symbol, signature, resolve,
                              infer, assignment, subset, typed_program
  typy-tyir/src/              module, function, block, instruction,
                              types, verify, print
  typy-lower/src/             lower, raise_analysis, ownership_analysis
  typy-codegen/src/           target, generate, layout, abi
  typy-build/src/             request, mode, emit, link_plan, linker,
                              artifact
  typy-driver/src/            pipeline
  typy-cli/src/               main, arguments, output, exit_status, guard
  typy-runtime/src/           entry, header, allocator, refcount, int,
                              float_repr, str, print, exception, stack
  typy-conformance/src/       main, test_case, oracle, verdict
tests/
  conformance/<area>/         <ID>_<behaviour>.py, one rule per file
  golden/<stage>/             expected --emit output
  e2e/                        tests that run the typy executable
```

## 3. Layering and code rules

The dependencies between crates are those of architecture section 4.2, and
a CI job checks them against `cargo metadata`. Within the code:

- Every compiler crate starts with `#![forbid(unsafe_code)]`. Only
  `typy-runtime` contains `unsafe`, and each block carries a comment that
  starts with `SAFETY:` and says why it is sound.
- Clippy runs with `-D warnings` and the `pedantic` group. Each allowed lint
  is listed in `Cargo.toml` with its reason.
- Identifiers of compiler objects are newtypes over `u32`: `FileId`,
  `SymbolId`, `ScopeId`, `FunctionId`, `BlockId`, `ValueId`. They index a
  vector owned by the aggregate that created them and are never mixed.
- Maps whose order can reach any output are `BTreeMap` or sorted `Vec`
  (architecture section 7.2). `HashMap` is used only for lookups whose
  order never escapes.
- Functions that can fail return `Result`. Compiler code does not call
  `unwrap`, `expect` or `panic!` on a path that input can reach; a broken
  internal invariant returns an internal `Diagnostic` with code `E9001`.
- There is no process-wide mutable state in the compiler. Everything a
  command needs travels in the `BuildRequest` and the objects the stages
  return.
- Line and branch coverage is measured with `cargo-llvm-cov` over the whole
  workspace, the runtime included, and counts the `typy` executable when
  tests run it as a subprocess. CI fails below 95 percent (NFR-11).

## 4. typy-diagnostics

### 4.1 Types

`TextOffset`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `0` | `u32` | none | A byte offset into a `SourceFile`; LEX-09 keeps it below 2^24 |

`Span`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `file` | `FileId` | none | Names a file of the current `Project` |
| `start` | `TextOffset` | none | On a UTF-8 character boundary |
| `end` | `TextOffset` | none | Not before `start`; on a character boundary |

`Span::new(file, start, end)` returns `None` when `end` is before `start`.

`Position`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `line` | `u32` | none | From 1 |
| `column` | `u32` | none | From 1, counted in code points (DIA-01) |

`DiagnosticCode`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `severity` | `Severity` | none | `Error` for `E` codes, `Warning` for `W` codes |
| `number` | `u16` | none | 0 to 9999; its first two digits give the range of section 4.3 |

`DiagnosticCode::parse("E0401")` returns `Err(CodeError::Malformed)` for
any text that is not a letter `E` or `W` followed by four digits.
`Display` writes the same form back.

`RuleId` wraps a `&'static str` such as `"TYP-03"`. Only the constants in
`rules.rs` create one, and a unit test checks that each constant is an ID
in `docs/requirements/srs.md`.

`Diagnostic`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `code` | `DiagnosticCode` | none | Listed in the registry (section 4.3) |
| `span` | `Span` | none | The primary location |
| `message` | `String` | none | One sentence in lower case with no final period (DIA-01) |
| `labels` | `Vec<Label>` | empty | Secondary spans, each with a short text |
| `fix` | `Option<Fix>` | `None` | Set only when the fix is certain (DIA-02) |
| `rule` | `Option<RuleId>` | `None` | Required when the diagnostic rejects valid Python (DIA-03) |

`Fix` holds a `Span` and the `replacement: String` for it. `Label` holds a
`Span` and a `text: String`.

`Diagnostics` collects diagnostics for one command.

| Method | Behaviour |
| --- | --- |
| `push(diagnostic)` | Adds it, unless one with the same code and span is already present |
| `has_errors()` | True when any diagnostic has severity `Error` |
| `into_sorted(lookup)` | Returns them sorted by file path, line, column and code |

`SourceLookup` is the trait the renderers use to turn a `Span` into text.
`typy-source::Project` implements it.

| Method | Returns |
| --- | --- |
| `path(file)` | The file's path relative to the project root |
| `position(file, offset)` | The `Position` of the offset |
| `line_text(file, line)` | The text of one line, without its line ending |

### 4.2 Rendering

`render_text(&Diagnostic, &dyn SourceLookup) -> String` writes the form
below. The source line is shown with a marker under the span, and the fix,
when present, as the corrected line.

```text
error[E0402]: `total` was declared as int and is assigned a str
  --> app.py:4:13
   |
 4 |     total = "done"
   |             ^^^^^^ this is a str
   |
   = rule: TYP-03
help: assign an int, or use a new variable
```

`render_json` is planned for 0.3 (DIA-05).

### 4.3 Diagnostic codes in 0.1

The registry in `registry.rs` lists every code with its summary, the rule
it enforces and its status. `typy explain` (0.3) reads the same table.

| Code | Summary | Rule |
| --- | --- | --- |
| E0101 | Source file larger than 16 MiB | LEX-09 |
| E0102 | Source file is not valid UTF-8 | LEX-01 |
| E0103 | Python syntax error, or syntax newer than Python 3.12 | STD-01 |
| E0104 | Imaginary literal | LEX-08 |
| E0201 | Feature planned for a later TyPy release | The feature's own rule |
| E0202 | Feature TyPy cannot compile with CPython's meaning | STD-04 |
| E0203 | Parameter without a type annotation | TYP-01 |
| E0204 | Function without a return annotation | TYP-01 |
| E0205 | Name is not defined | STD-04 |
| E0206 | Import of a module TyPy does not provide | LIB-08 |
| E0301 | `is` used on values other than `None` or class instances | EXP-08 |
| E0302 | `int ** int` with an exponent that is not a non-negative literal | EXP-04 |
| E0303 | `and` or `or` with operands of different types | EXP-09 |
| E0304 | Truth test on a value of an unsupported type | EXP-10 |
| E0305 | Conditional expression with branches of different types | EXP-11 |
| E0306 | Variable read before it is assigned on every path | TYP-12 |
| E0401 | Operator not defined for these operand types | EXP-02 |
| E0402 | Assignment changes the type of a variable | TYP-03 |
| E0403 | `Any`, `cast` or an untyped value | TYP-10 |
| E0404 | Argument of the wrong type | TYP-01 |
| E0405 | Returned value of the wrong type | TYP-01 |
| E0406 | Arithmetic on `bool` | DAT-03 |
| E0407 | Comparison between unsupported types | EXP-06 |
| E0701 | C toolchain program not found | A-01 |
| E0702 | Link failed | A-01 |
| E9001 | Internal compiler error | NFR-09 |

## 5. typy-source

### 5.1 Types

`ProjectSettings`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `root` | `Utf8PathBuf` | Directory of the main file | Absolute; every module path is relative to it |
| `release` | `Release` | `Release::V0_1` | The release whose feature table the subset checker uses |
| `python_floor` | `PythonFloor` | `PythonFloor::V3_12` | One of 3.12 to 3.15; the parser's target version and the lowest oracle version (ADR-0008). Read from `typy.toml` from 0.3 (CLI-12) |

Module search paths, links and exports from `typy.toml` are planned for 0.3
and 0.5 (CLI-07, FFI-04, FFI-05).

`SourceFile`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `id` | `FileId` | none | Unique in its `Project` |
| `path` | `Utf8PathBuf` | none | Relative to `ProjectSettings::root`, or `<stdlib>/name.py` for embedded modules |
| `text` | `Arc<str>` | none | Valid UTF-8, at most 16 MiB, without a leading byte order mark |
| `lines` | `LineIndex` | none | Byte offset of each line start, built when the file is read |

`SourceFile::read(id, root, path)` refuses a file over 16 MiB with E0101
before reading it, and a file that is not UTF-8 with E0102 at the offset of
the first invalid byte. A leading byte order mark is removed, as CPython
accepts it.

`ModuleName` wraps a `String` of dotted identifiers, such as `__main__` or
`builtins`. `ModuleName::new` refuses an empty name and any part that is
not an identifier.

`Module`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `name` | `ModuleName` | none | `__main__` for the main file (STD-06) |
| `file` | `FileId` | none | Its `SourceFile` |
| `syntax` | `SyntaxTree` | none | Built by `parse` |

`Project`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `settings` | `ProjectSettings` | none | Immutable after load |
| `main` | `ModuleName` | `__main__` | A key of `modules` |
| `modules` | `BTreeMap<ModuleName, Module>` | none | The main module and `builtins` in 0.1 |
| `files` | `Vec<SourceFile>` | none | Indexed by `FileId` |

`Project::load(main_path, settings) -> Result<Project, Diagnostics>` reads
the main file and the embedded `builtins` module, parses both, and returns
every diagnostic of every file together. Imports between modules are
planned for 0.3 (MOD-01); in 0.1 an `import` statement gives E0201.

### 5.2 The syntax tree

`SyntaxTree` is TyPy's own copy of the parsed module. Its node types follow
the names of CPython's `ast` module for Python 3.12, such as `FunctionDef`,
`AnnAssign`, `BinOp` and `Compare`, so the Language Reference and the `ast`
documentation describe them. Every node carries a `Span`. No type from
`ruff_python_parser` or `ruff_python_ast` appears outside `parse.rs`
(ADR-0005).

`parse(file: &SourceFile) -> Result<SyntaxTree, Vec<Diagnostic>>` calls
`ruff_python_parser::parse_unchecked` with
`ParseOptions::from(Mode::Module)` and the program's Python floor as the
target version, 3.12 in 0.1. Each entry of `Parsed::errors()` and
`Parsed::unsupported_syntax_errors()` becomes an E0103 diagnostic, so
syntax that the floor version does not accept is refused (STD-01). The
parser already reads PEP 810 lazy imports and marks them `is_lazy`; they
reach the syntax tree in 0.3 (MOD-06). The tree is converted only when
there are no errors.

## 6. typy-stdlib

`typy-stdlib` embeds each module's source with `include_str!` and offers
`module_source(name) -> Option<&'static str>` and `module_names()`, sorted.
In 0.1 it holds `builtins.py`, written in the 0.1 subset over the
primitives that `_typy.pyi` declares, such as
`_typy.write_stdout(text: str) -> None`. Only modules inside `typy-stdlib`
can import `_typy` (ADR-0007).

## 7. typy-check

### 7.1 Types

`Type` is an enum. In 0.1 its variants are `Int`, `Float`, `Bool`, `Str`,
`NoneType`, `Never` and `Error`. `Error` is the type of an expression that
already has a diagnostic; every check accepts it silently, so one cause
gives one error (DIA-04). `List`, `Tuple`, `Dict`, `Set`, `Bytes` and
`Class` are planned for 0.2; `Union`, `TypeParameter` and `Callable` for
0.4 and 1.0.

| Method | Behaviour |
| --- | --- |
| `is_assignable_to(&self, target)` | True when equal, or `Int` to `Float` (TYP-09), or either side is `Error` |
| `display(&self)` | The Python spelling, such as `int` or `None` |

`Symbol`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `id` | `SymbolId` | none | Unique in the `SymbolTable` |
| `name` | `Name` | none | A Python identifier |
| `kind` | `SymbolKind` | none | `Local`, `Parameter`, `ModuleVariable`, `Function` or `Builtin` |
| `ty` | `Type` | none | Never changes after the symbol is created (TYP-03) |
| `span` | `Span` | none | Where the name is first bound |

`Scope`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `id` | `ScopeId` | none | Unique in the `SymbolTable` |
| `kind` | `ScopeKind` | none | `Module` or `Function` in 0.1 |
| `parent` | `Option<ScopeId>` | `None` for a module | A function scope's parent is its module |
| `names` | `BTreeMap<Name, SymbolId>` | empty | One symbol per name |

`FunctionSignature`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `parameters` | `Vec<Parameter>` | empty | Each with `name`, `ty` and `default: Option<Constant>` |
| `returns` | `Type` | none | `NoneType` for `-> None` |

A default value is a constant in 0.1, which makes "evaluated once, when the
function is defined" (EXP-16) hold trivially.

`TypedProgram`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `symbols` | `SymbolTable` | none | All scopes and symbols |
| `functions` | `Vec<TypedFunction>` | none | Sorted by qualified name; the module body is one of them |
| `entry` | `FunctionId` | none | The body of `__main__` |

A `TypedFunction` holds its `FunctionSignature` and its body as typed
statements, in which every expression is a `TypedExpr` with a `kind`, a
`ty` and a `span`.

### 7.2 Behaviour

`TypeChecker::check(project) -> Result<TypedProgram, Diagnostics>` runs, for
each module in name order:

1. `SubsetChecker::check` walks the syntax tree and looks up each node kind
   in the feature table of `ProjectSettings::release`. A node planned for a
   later release gives E0201, naming the release and the rule, such as
   "lists are planned for TyPy 0.2 (DAT-07)". A node that is excluded from
   TyPy gives E0202 with rule STD-04.
2. The resolver binds names. A name assigned anywhere in a function is a
   local of that function, as LR 4.2.2 defines. In 0.1 a function may read
   module-level functions and builtins, but not module-level variables;
   that needs `global` and is planned for 0.3 (SMT-08).
3. Inference gives each local the type of its first assignment (TYP-02) and
   refuses a later assignment of another type with E0402.
4. The definite assignment pass follows every path through each function
   and the module body, and refuses a read that some path reaches with the
   variable unassigned, with E0306 (TYP-12).
5. Expressions are checked against the rules EXP-01 to EXP-11.

Functions are checked in parallel. Their diagnostics are merged and sorted
before they are returned (architecture section 7.5).

The 0.1 feature table accepts: module-level `def` without decorators or
type parameters; positional and keyword parameters with constant defaults;
`return`, `pass`, `break`, `continue`; assignment, annotated assignment and
augmented assignment to a name; `if`, `while` and `for` with `else`, where
`for` iterates over a call to `range` with one to three `int` arguments;
expression statements; the operators of EXP-02 to EXP-11; calls; names;
constants of type `int`, `float`, `str`, `bool` and `None`; and indexing a
`str` with an `int`.

## 8. typy-tyir

### 8.1 Types

`TyirType` is `Int`, `Float`, `Bool`, `Unit` or `Object(TypeId)`. An `Int`
is a 64-bit word: an inline value has its lowest bit set and holds the
number shifted left by one, so it covers -2^62 to 2^62 - 1; any other value
is a pointer to a `BigInt` (ADR-0001, DAT-01). `Str` values are
`Object(TypeId::STR)`.

`ValueInfo` holds a value's `ty: TyirType` and its
`representation: ValueRepresentation`: `Inline`, `Stack` or `Heap`. In 0.1
`Int`, `Float`, `Bool` and `Unit` are `Inline` and objects are `Heap`;
`Stack` is chosen by escape analysis from 0.4 (MEM-03).

`TyirFunction`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `id` | `FunctionId` | none | Unique in the module |
| `symbol` | `String` | none | The mangled name of section 10.3 |
| `parameters` | `Vec<ValueId>` | empty | The entry block's parameters |
| `return_type` | `TyirType` | none | `Unit` for `-> None` |
| `may_raise` | `bool` | `false` | Set by `RaiseAnalysis` |
| `blocks` | `Vec<BasicBlock>` | none | The first is the entry block |
| `values` | `Vec<ValueInfo>` | empty | Indexed by `ValueId` |

`BasicBlock` has an `id`, its `parameters: Vec<ValueId>`, its
`instructions: Vec<Instruction>` and one `terminator: Terminator`.

`Instruction` has an optional `result: ValueId`, an `operation: Operation`
and the `span` of the source it came from, which becomes the line of a
`TracebackEntry`. `Operation` covers constants, arithmetic and comparison
on each `TyirType`, calls to runtime functions that cannot raise,
`Increment(value)` and `Decrement(value)`.

`Terminator`

| Variant | Meaning |
| --- | --- |
| `Jump { target, arguments }` | Continue in `target` |
| `Branch { condition, then, otherwise }` | Continue in one of two blocks |
| `Invoke { callee, arguments, normal, error }` | Call a function that may raise; continue in `normal` with the result, or in `error` with the exception |
| `Return(value)` | Return normally |
| `ReturnError(exception)` | Return the exception to the caller (ADR-0004) |
| `Unreachable` | After a call that cannot return |

### 8.2 Verifier and text form

`Verifier::verify(module) -> Result<(), Diagnostic>` checks that every
value is defined once and before each use on every path; that block
arguments match block parameters in number and type; that every operation
has operands of the types it takes; that `Invoke` is used exactly for
callees marked `may_raise`; and that a function not marked `may_raise`
contains no `ReturnError`. A failure is an E9001 diagnostic that names the
function, the block and the pass that ran last. The verifier runs after
every pass in every build of the compiler.

`print(module) -> String` writes the deterministic text form used by
`--emit=tyir` and `--emit=tyir-mem`:

```text
fn _TY8__main__3div(v0: int, v1: int) -> int raises {
b0(v0: int, v1: int):
  v2 = int.eq v1, 0
  branch v2, b1, b2
b1:
  v3 = call rt.zero_division_error()
  return_error v3
b2:
  v4 = int.floordiv v0, v1
  return v4
}
```

## 9. typy-lower

`Lowering::lower(program) -> Result<TyirModule, Diagnostic>` turns each
`TypedFunction` into a `TyirFunction`, then runs the passes below, with the
verifier after each one. Its only errors are internal, E9001.

| Pass | Behaviour |
| --- | --- |
| Lowering | Statements become blocks; `for` over `range` becomes a counting loop; chained comparisons evaluate each operand once (EXP-06); `and` and `or` become branches (EXP-09) |
| `RaiseAnalysis` | Marks a function `may_raise` when it contains an operation that may raise, such as integer division (EXP-03) or a `str` index (DAT-11), or calls a function that is marked. It iterates over the call graph until nothing changes. Calls to marked functions become `Invoke` |
| `OwnershipAnalysis` | Inserts `Increment` and `Decrement` for heap values so that each owned reference is released once on every path, including `error` successors. Parameters are borrowed; the last use of a value moves it (MEM-04) |
| Stack check | Puts a stack-limit check at the start of each function that is part of a cycle in the call graph (FUN-02) |

`EscapeAnalysis` is planned for 0.4 and `CycleAnalysis` for 1.0.

## 10. typy-codegen

### 10.1 Types

`TargetTriple` is one of the supported triples. In 0.1 they are
`x86_64-unknown-linux-gnu` and `x86_64-unknown-linux-musl`. `Target::host()`
returns the triple `typy` was built for. Parsing any other triple gives a
usage error with the list of supported triples.

`ObjectFile` holds its `target` and its `bytes: Vec<u8>`.

`CodeGenerator::generate(module, target, mode) -> Result<ObjectFile,
Diagnostic>` creates one LLVM context and module, emits each function in
the order of `TyirModule::functions`, verifies the LLVM module, runs the
optimisation pipeline of the mode (`-O0`, `-O2` or `-Oz`) and writes the
object file to memory. Its only errors are internal, E9001.

### 10.2 Contract with the runtime

| Item | Form |
| --- | --- |
| Object header | 8 bytes: `reference_count: u32`, then `type_id: u32` |
| Type table | `__typy_types`, an array of `TypeDescriptor` indexed by `type_id`. The runtime owns IDs 0 to 255; the compiler numbers program types from 256 |
| A function that may raise | Returns `{ result, *mut ExceptionObject }`; a null pointer means no exception |
| A function that cannot raise | Returns its result directly |
| Entry | The runtime's `main` calls `__typy_main`, the body of `__main__` |
| Runtime functions | Named `typy_rt_*`, declared with the C calling convention |

### 10.3 Symbol names

A TyPy function's symbol is `_TY`, then the length and text of each part of
its module name, then the length and text of the function name:
`_TY8__main__3fib`. Symbols are internal and may change in any release.

## 11. typy-build

`BuildMode` is `Debug`, `Release` or `ReleaseSmall`. `EmitStage` is `Ast`,
`TypedAst`, `Tyir`, `TyirMem`, `LlvmIr`, `Asm` or `Obj`; `EmitStage::parse`
accepts the lower-case names of CLI-04.

`BuildRequest`

| Field | Type | Default | Rule |
| --- | --- | --- | --- |
| `main` | `Utf8PathBuf` | none | A `.py` file that exists |
| `mode` | `BuildMode` | `Debug` | `--release` and `--release-small` are exclusive |
| `target` | `Target` | `Target::host()` | Supported (section 10.1) |
| `emit` | `Option<EmitStage>` | `None` | When set, no executable is written |
| `output` | `Option<Utf8PathBuf>` | `None` | Defaults to the main file's name without `.py`, next to it |
| `link` | `LinkPlan` | empty | Libraries and exports, planned for 0.5 |
| `deny_warnings` | `bool` | `false` | DIA-07 |

`Linker::link(object, plan, output) -> Result<Artifact, Diagnostic>`
writes the object to a temporary file, then runs the C toolchain as a
subprocess with an argument list and no shell:

| Target | Program | Arguments |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `cc` | `-o TEMP OBJECT RUNTIME_ARCHIVE -lm -Wl,--gc-sections` |
| `x86_64-unknown-linux-musl` | `musl-gcc` | The same, with `-static` |

On success it renames the temporary output to `output`, so a failed link
leaves no file (UC-02, flow 5a). A missing program gives E0701 with the
program name; a failed link gives E0702 with the arguments and the
linker's standard error. An `Artifact` holds its `path` and its `kind`.

## 12. typy-driver

`Pipeline::run(request) -> Result<Outcome, Failure>` loads the `Project`,
checks it, lowers it, generates code and links, in that order, and stops
after `request.emit` when it is set. `Outcome` holds the artifacts and any
warnings; `Failure` holds the diagnostics. `Pipeline::check(path)` stops
after checking. The driver holds no state between calls, so the conformance
suite runs many builds in one process.

## 13. typy-cli

| Item | Design |
| --- | --- |
| Arguments | Parsed with `clap`; the commands `check`, `build` and `run` in 0.1, `explain` in 0.3 |
| Logging | `--log-level` sets a `tracing` subscriber for the target `typy`, writing to standard error; off by default |
| Output | Diagnostics rendered with `render_text` to standard error; `--emit` output to standard output or `-o` |
| Exit status | `ExitStatus` is `Success = 0`, `ProgramErrors = 1`, `Usage = 2`, `Internal = 101` (CLI-06) |
| Panic guard | The command runs inside `std::panic::catch_unwind`, with a panic hook that records the location; a panic becomes E9001 and exit status 101 |
| `typy run` | Builds into `cache/<key>/` under `TYPY_HOME`, where `key` is the SHA-256 of the compiler version, the request and the source texts; then starts the executable with the arguments after `--` and exits with its status |

## 14. typy-runtime

### 14.1 Types

| Type | Layout | Rule |
| --- | --- | --- |
| `ObjectHeader` | `reference_count: u32`, `type_id: u32` | The first 8 bytes of every heap object (NFR-12) |
| `TypeDescriptor` | `size: u32`, `flags: u32`, `drop: unsafe extern "C" fn(*mut ObjectHeader)`, `name: *const u8` | Generated by the compiler or built into the runtime; never changed |
| `Int` | One `u64` word, as in section 8.1 | Arithmetic on two inline values that does not overflow never calls the runtime |
| `BigInt` | Header, `sign_and_length: i32`, then that many `u64` limbs, least significant first | Normalised: no leading zero limb, and a value that fits inline is never a `BigInt` |
| `Str` | Header, `length: u64` in code points, `kind: StrKind`, then the data | `kind` is `Latin1`, `Ucs2` or `Ucs4`, the narrowest that holds every code point, as in PEP 393 |
| `ExceptionObject` | Header, `message: *mut Str`, `traceback: TracebackBuffer`, `cause: *mut ExceptionObject` | Its `type_id` names its exception class |
| `TracebackEntry` | `function: *const u8`, `file: *const u8`, `line: u32` | One per frame passed through |

![Design classes of the runtime](../diagrams/class-design-runtime.svg)

### 14.2 Behaviour

| Module | Behaviour |
| --- | --- |
| `entry` | `main` records the stack limit, calls `__typy_main`, reports an uncaught exception, flushes standard output and returns the exit status |
| `allocator` | `typy_rt_allocate`, `typy_rt_reallocate` and `typy_rt_free` over `malloc`, `realloc` and `free`. Running out of memory prints `MemoryError` and aborts |
| `refcount` | `typy_rt_increment` and `typy_rt_decrement`, not atomic (A-03). A decrement to zero calls the type's `drop`, then frees the memory |
| `int` | Addition, subtraction, multiplication, floor division, modulo, power, negation, comparison, conversion to and from `str` and `float`, all with Python's results (EXP-02, EXP-05) |
| `float_repr` | The shortest digit string that reads back as the same double, formatted by CPython's rules for `repr`: fixed notation when the decimal exponent is from -4 to 15, otherwise scientific with a sign and at least two exponent digits, and `inf`, `-inf`, `nan` and `-0.0` as CPython writes them (DAT-02) |
| `str` | Creation, concatenation, repetition, indexing with negative indexes (DAT-11), length, comparison, `repr` |
| `print` | Standard output is buffered in 8 KiB and flushed when full, at exit, and before anything is written to standard error |
| `exception` | Creates the built-in exceptions of EXC-02 with CPython's message for each operation, such as `division by zero` for `1 / 0`, `integer division or modulo by zero` for `1 // 0` and `float modulo by zero` for `1.0 % 0.0`. Each message has a conformance test; where the oracle versions word a message differently, that test is invalid (STD-03). It adds `TracebackEntry` values, and it prints an uncaught exception as `Traceback (most recent call last):`, one line per entry, then `Type: message`, or `Type` alone when the message is empty, as CPython does (STD-05) |
| `stack` | The check at the start of recursive functions compares the stack pointer with the recorded limit, which leaves 64 KiB unused, and raises `RecursionError: maximum recursion depth exceeded` |

The runtime uses no `core::fmt`. Each of its `unsafe` blocks is covered by
tests under AddressSanitizer and Miri (ADR-0006).

## 15. typy-conformance

A conformance test is a Python file that CPython can run, with its
expectations in comments at the top:

```python
# typy-test: EXP-03
# typy-expect: accepted
def ratio(a: int, b: int) -> float:
    return a / b

print(ratio(1, 4))
print(ratio(1, 0))
```

```python
# typy-test: EXP-04
# typy-expect: rejected E0302 at 4:12
n: int = 3
print(2 ** n)
```

| Field | Rule |
| --- | --- |
| `typy-test` | One or more requirement IDs, separated by commas; each exists in the specification |
| `typy-expect` | `accepted`, or `rejected CODE at LINE:COLUMN` |
| `typy-python` | Optional; the test's Python floor, `3.12` when absent |

The file name starts with the first requirement ID, such as
`EXP-03_float_division_by_zero_raises.py`. For an accepted test, the suite
runs `python3.X -I -X utf8 FILE` for each oracle version from the test's
Python floor up, builds the file
with `Pipeline::run` in debug and in release mode, and compares standard
output, exit status and the last line of standard error (UC-09). The
second test above is a valid Python program that CPython runs; TyPy rejects
it because the exponent is a variable.

## 16. Design class diagrams

![Design classes of typy_diagnostics and typy_source](../diagrams/class-design-source.svg)

![Design classes of typy_check](../diagrams/class-design-check.svg)

![Design classes of typy_tyir and typy_lower](../diagrams/class-design-tyir-and-lowering.svg)

![Design classes of typy_codegen and typy_build](../diagrams/class-design-codegen-and-build.svg)

The dependencies between these crates are shown in the building blocks
diagram of architecture section 4.1.

## 17. Traceability for release 0.1

Each requirement whose release is 0.1, and the sections of this document
that meet it. The test plan traces the same requirements to their tests.

| Requirement | Sections |
| --- | --- |
| STD-01 | 5.2 |
| STD-02 | 14.2, 15 |
| STD-03 | 15 |
| STD-04 | 7.2 |
| STD-06 | 5.1, 14.2 |
| STD-07 | 6 |
| CLI-01 | 11, 13 |
| CLI-02 | 13 |
| CLI-03 | 12, 13 |
| CLI-04 | 8.2, 11, 12 |
| CLI-05 | 10.1, 11 |
| CLI-06 | 13 |
| CLI-11 | 13 |
| LEX-01 | 5.1 |
| LEX-02 | 5.2 |
| LEX-03 | 5.2 |
| LEX-04 | 5.2 |
| LEX-05 | 5.2 |
| LEX-08 | 4.3, 7.2 |
| LEX-09 | 5.1 |
| TYP-01 | 7.1, 7.2 |
| TYP-02 | 7.2 |
| TYP-03 | 7.1, 7.2 |
| TYP-04 | 7.2 |
| TYP-09 | 7.1 |
| TYP-10 | 4.3, 7.1 |
| TYP-11 | 7.1, 8.1 |
| TYP-12 | 7.2 |
| DAT-01 | 8.1, 14.1 |
| DAT-02 | 14.2 |
| DAT-03 | 4.3, 7.2 |
| DAT-04 | 7.1, 8.1 |
| DAT-05 | 14.1, 14.2 |
| EXP-01 | 9 |
| EXP-02 | 9, 14.2 |
| EXP-03 | 9, 14.2 |
| EXP-04 | 4.3 |
| EXP-05 | 14.2 |
| EXP-06 | 4.3, 9 |
| EXP-08 | 4.3 |
| EXP-09 | 4.3, 9 |
| EXP-10 | 4.3 |
| EXP-11 | 4.3 |
| EXP-16 | 7.1 |
| SMT-01 | 7.2 |
| SMT-03 | 7.2 |
| CST-01 | 7.2, 9 |
| CST-02 | 7.2, 9 |
| FUN-01 | 7.2 |
| FUN-02 | 9, 14.2 |
| LIB-01 | 6 |
| LIB-09 | 6 |
| MEM-01 | 14.2 |
| DIA-01 | 4.1 |
| DIA-02 | 4.1, 4.2 |
| DIA-03 | 4.1, 7.2 |
| DIA-04 | 4.1, 7.1 |
| DIA-06 | 4.3 |
| DIA-07 | 11 |
| NFR-01 | 11, 14.2 |
| NFR-02 | 14.2 |
| NFR-06 | 3, 10.1 |
| NFR-07 | 3, 14.2 |
| NFR-08 | 9 |
| NFR-09 | 3, 13 |
| NFR-10 | 15 |
| NFR-11 | 3 |
| NFR-15 | 9, 10.1 |
