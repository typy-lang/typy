# ADR-0016: Compile each module separately and verify interfaces at link time

## Status

Proposed

## Date

2026-11-11

## Context

TyPy compiles a program as a whole (ADR-0015): the checker reads every
module, lowering numbers tuple types, globals and classes across the
program, and one LLVM module becomes one object file. That is simple, and
LLVM optimises across modules, but every build does all the work again. A
change to one function body rebuilds every module.

Long builds are the disadvantage of C and C++ that slows their development
cycle most. Their cause is known: each `#include` is parsed again in every
file that includes it, template code is compiled again in each file, and a
header edit rebuilds everything that includes it. Go was designed for fast
builds and avoids all three; OCaml and Swift compile each module against
compact interfaces of the modules it imports.

TyPy aims to serve programs at every scale: a fast cycle from edit to run
while developing, and small, fast, stripped binaries for release.

Today's design depends on the whole program in five places:

1. The checker needs each imported module's signatures, class layouts and
   module variable types, which it gets by checking that module.
2. Lowering finds which functions may raise by iterating over the whole
   program until the set stops growing.
3. The call graph that decides which functions check the stack is built
   from the whole program.
4. Tuple type descriptors, globals (`typy.global.N`), module objects and
   class indices are numbered across the program.
5. LLVM inlines and optimises across modules because they are one LLVM
   module.

## Options considered

For how modules are compiled:

1. **Keep whole-program compilation.** Simplest, and the best code, but the
   build time grows with the program, not with the change.
2. **Compile each module separately against the interfaces of its imports,
   and link the objects.** A changed body recompiles one module; a changed
   interface recompiles the modules that import it.
3. **Cache the whole-program build only.** Fast when nothing changed, and
   no faster for any change.

For the files a separate build writes, Python's conventions were weighed
against ADR-0003, under which TyPy adds no names of its own:

1. **`.pyi` for an interface.** A PEP 484 stub file holds typed signatures,
   which is what an interface is, and type checkers and editors read it.
2. **`.pyo` or `.pyd` for compiled code.** `.pyo` was bytecode, removed by
   PEP 488 in Python 3.5; `.pyd` is a Windows extension module that CPython
   loads. Either would tell tools, and CPython itself, something false.
3. **The native toolchain's extensions for compiled code.** `.o` for an
   object, `.a` for a static library and `.so` for a shared one are what
   linkers and debuggers expect.

For where they go, a directory of TyPy's own, such as `.typy/`, would be a
name of TyPy's own. PEP 3147 already gives every Python implementation a
place: `__pycache__/`, each file tagged with the implementation that wrote
it, as CPython writes `shapes.cpython-312.pyc`. A stub beside its source,
`shapes.pyi`, would be read by type checkers in place of the source.

## Decision

TyPy will use option 2 for development builds, with `.pyi` interfaces and
native object files in `__pycache__/`, and will keep whole-program code
generation for release builds.

- **Interfaces.** Checking a module writes its interface as a stub file:
  `__pycache__/<module>.typy-<release>.pyi`, such as
  `shapes.typy-05.pyi`. It is valid PEP 484 Python: each function's
  signature, each class with its attributes in layout order, and each
  module variable with its type. A leading comment block holds what a stub
  cannot say: the interface's hash, which functions may raise, and the
  release that wrote it. Checking a module reads the interfaces of its
  imports, never their source.
- **Objects.** Compiling a module writes
  `__pycache__/<module>.typy-<release>-<target>-<mode>.o`. Its exported
  symbols are mangled with the module's name, as they are today. Every
  name that is numbered across the program today becomes a symbol of the
  module that defines it: globals, module objects, module states and class
  type objects. A tuple type descriptor becomes a link-once symbol named
  after its layout, so modules that use one tuple type share one.
- **Whole-program facts at module boundaries.** Whether a function may
  raise is part of its interface. A call into another module checks the
  stack, since the caller cannot see whether it recurses.
- **Rebuilding.** A module is recompiled when its source, the interface of
  an import it compiled against, the compiler, the mode or the target
  changes. A body that changes without changing its interface recompiles
  that module alone, and modules that do not depend on each other compile
  in parallel.
- **Verified linking.** Each object records the hash of every interface it
  compiled against. TyPy's link step compares them with the interfaces of
  the objects it links before it calls the system linker. A mismatch is an
  error that names the modules, such as "`app` was compiled against an
  older `shapes`", and never a program that crashes at run time.
- **Release builds.** A release build keeps generating code for the whole
  program, or uses ThinLTO across the objects, so cross-module inlining is
  not lost, and strips debug information. Development builds skip the
  costly optimisations and keep debug information.
- **Linking.** Static linking comes first. Dynamic linking of TyPy modules
  needs a stable ABI for object layouts, exception state and type
  identity, and one shared runtime; it waits until that ABI is fixed after
  1.0 (OI-23).

## Consequences

- An edit that changes no interface rebuilds one module and relinks, so the
  development cycle grows with the change, not with the program.
- Interfaces are readable Python, which a person, a type checker or an
  editor can open.
- `__pycache__/` gains TyPy's files beside CPython's, each tagged, so the
  two never read each other's files, and `.gitignore` already leaves them
  out.
- Development builds lose cross-module inlining, and pay for a stack check
  on each call into another module; release builds keep today's code.
- The lowering of the five whole-program facts above changes, which is the
  bulk of the work. Until it is done, new work avoids adding more names
  numbered across the program.
- NFR-04 gains targets for an incremental rebuild and for compile speed,
  with benchmarks, when this record is accepted.

## Related requirements

NFR-02, NFR-04, NFR-06, MOD-01, MOD-03, STD-07, ADR-0003, ADR-0005,
ADR-0007, ADR-0015.
