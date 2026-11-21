# Architecture decision records

Each record states one decision, with the options considered and the
consequences. An accepted record is not rewritten; a later record
supersedes it.

| ADR | Decision | Status | Date |
| --- | --- | --- | --- |
| [ADR-0001](0001-use-optimised-reference-counting-with-cycle-collection.md) | Use optimised reference counting with selective cycle collection | Accepted | 2026-11-05 |
| [ADR-0002](0002-accept-only-standard-python-with-cpython-meaning.md) | Accept only standard Python, with the meaning CPython gives it | Superseded by ADR-0008 | 2026-11-05 |
| [ADR-0003](0003-implement-python-independently-with-no-names-of-its-own.md) | Implement Python independently, with no names of its own | Accepted | 2026-11-05 |
| [ADR-0004](0004-compile-exceptions-as-error-values-returned-to-the-caller.md) | Compile exceptions as error values returned to the caller | Accepted | 2026-11-05 |
| [ADR-0005](0005-write-the-compiler-in-rust-with-ruff-parser-and-llvm-23.md) | Write the compiler in Rust, with Ruff's parser and LLVM 23 | Accepted | 2026-11-05 |
| [ADR-0006](0006-write-the-runtime-in-rust-without-the-standard-library.md) | Write the runtime in Rust without the standard library | Accepted | 2026-11-06 |
| [ADR-0007](0007-write-standard-library-modules-in-python-compiled-by-typy.md) | Write standard library modules in Python, compiled by TyPy | Accepted | 2026-11-06 |
| [ADR-0008](0008-judge-each-program-from-its-own-python-floor.md) | Judge each program by CPython from its own Python floor | Accepted | 2026-11-06 |
| [ADR-0009](0009-hold-collections-in-the-runtime-and-measure-what-programs-link.md) | Hold the built-in collections in the runtime, and measure the runtime a program links | Accepted | 2026-11-08 |
| [ADR-0010](0010-hash-and-order-sets-and-dicts-as-cpython-does.md) | Hash and order sets and dicts as CPython does | Accepted | 2026-11-09 |
| [ADR-0011](0011-hold-none-in-a-value-each-type-never-takes.md) | Hold None in a value each type never takes | Accepted | 2026-11-09 |
| [ADR-0012](0012-root-every-type-at-object-with-a-type-object-for-each.md) | Root every type at object, with a type object for each | Accepted | 2026-11-10 |
| [ADR-0013](0013-make-exceptions-objects-of-exception-classes.md) | Make exceptions objects of exception classes | Accepted | 2026-11-10 |
| [ADR-0014](0014-iterate-with-runtime-iterator-objects-and-the-iterator-protocol.md) | Iterate with runtime iterator objects and the iterator protocol | Accepted | 2026-11-11 |
| [ADR-0015](0015-compile-modules-into-one-program-with-module-objects.md) | Compile modules into one program, with module objects | Accepted | 2026-11-11 |
| [ADR-0016](0016-compile-each-module-separately-and-verify-interfaces-at-link-time.md) | Compile each module separately and verify interfaces at link time | Proposed | 2026-11-11 |
| [ADR-0017](0017-warn-where-cpython-warns-while-compiling.md) | Warn where CPython warns while compiling | Superseded by ADR-0033 | 2026-11-12 |
| [ADR-0018](0018-provide-math-and-sys-as-checker-tables-over-runtime-functions.md) | Provide math and sys as checker tables over runtime functions | Accepted | 2026-11-12 |
| [ADR-0019](0019-open-text-files-over-buffered-descriptors-with-cpythons-errors.md) | Open text files over buffered descriptors with CPython's errors | Accepted | 2026-11-12 |
| [ADR-0020](0020-bound-nesting-as-cpython-does-and-compile-on-a-large-stack.md) | Bound nesting as CPython does and compile on a large stack | Accepted | 2026-11-13 |
| [ADR-0021](0021-extend-the-base-layout-and-call-overrides-through-a-method-table.md) | Extend the base layout and call overrides through a method table | Accepted | 2026-11-13 |
| [ADR-0022](0022-hold-a-union-of-objects-as-a-pointer-and-others-as-a-tag-and-value.md) | Hold a union of objects as a pointer and others as a tag and value | Superseded by ADR-0027 | 2026-11-13 |
| [ADR-0023](0023-compile-generics-and-protocol-parameters-to-one-copy-per-concrete-type.md) | Compile generics and protocol parameters to one copy per concrete type | Accepted | 2026-11-13 |
| [ADR-0024](0024-keep-class-variables-in-globals-and-defaults-in-each-instance.md) | Keep class variables in globals and defaults in each instance | Accepted | 2026-11-13 |
| [ADR-0025](0025-compile-a-class-method-once-for-each-class-that-has-it.md) | Compile a class method once for each class that has it | Accepted | 2026-11-13 |
| [ADR-0026](0026-call-operator-methods-directly-and-from-the-type-object-slots.md) | Call operator methods directly and from the type object slots | Accepted | 2026-11-13 |
| [ADR-0027](0027-hold-a-union-as-its-object-or-as-a-tuple-of-a-tag-and-a-slot-per-member.md) | Hold a union as its object, or as a tuple of a tag and a slot per member | Accepted | 2026-11-13 |
| [ADR-0028](0028-make-each-copy-of-a-generic-class-answer-as-the-generic-class.md) | Make each copy of a generic class answer as the generic class | Accepted | 2026-11-14 |
| [ADR-0029](0029-report-cpython-3-15-as-the-python-version-of-a-built-program.md) | Report CPython 3.15 as the Python version of a built program | Accepted | 2026-11-14 |
| [ADR-0030](0030-read-exception-args-and-exit-codes-where-a-program-uses-them.md) | Read exception args and exit codes where a program uses them | Accepted | 2026-11-14 |
| [ADR-0031](0031-type-a-file-by-its-literal-mode-and-a-mode-value-as-text.md) | Type a file by its literal mode, and a mode value as text | Accepted | 2026-11-14 |
| [ADR-0032](0032-export-a-module-as-a-c-library-described-by-its-own-annotations.md) | Export a module as a C library described by its own annotations | Accepted | 2026-11-15 |
| [ADR-0033](0033-warn-about-a-match-that-leaves-members-of-its-subject-unmatched.md) | Warn about a match that leaves members of its subject unmatched | Accepted | 2026-11-15 |
| [ADR-0034](0034-type-exception-args-by-the-raises-that-create-them.md) | Type exception args by the raises that create them | Accepted | 2026-11-15 |
| [ADR-0035](0035-type-os-error-attributes-by-the-raises-that-create-them.md) | Type OSError attributes by the raises that create them | Accepted | 2026-11-15 |
| [ADR-0036](0036-call-the-function-the-last-run-import-bound.md) | Call the function the last-run import bound | Accepted | 2026-11-15 |
| [ADR-0037](0037-reach-class-methods-and-class-variables-through-the-instances-class.md) | Reach class methods and class variables through the instance's class | Accepted | 2026-11-15 |
| [ADR-0038](0038-give-way-to-the-next-operator-method-on-not-implemented.md) | Give way to the next operator method on NotImplemented | Accepted | 2026-11-16 |
| [ADR-0039](0039-keep-a-slot-methods-exception-until-the-runtime-call-returns.md) | Keep a slot method's exception until the runtime call returns | Accepted | 2026-11-16 |
| [ADR-0040](0040-derive-each-copy-of-a-generic-class-from-the-copy-of-its-base.md) | Derive each copy of a generic class from the copy of its base | Accepted | 2026-11-16 |
| [ADR-0041](0041-hold-a-protocol-value-as-an-object-and-call-it-through-a-dispatcher.md) | Hold a protocol value as an object and call it through a dispatcher | Accepted | 2026-11-16 |
| [ADR-0042](0042-allocate-an-instance-that-does-not-escape-in-its-functions-frame.md) | Allocate an instance that does not escape in its function's frame | Accepted | 2026-11-17 |
| [ADR-0043](0043-name-the-allocator-in-typy-toml-and-call-it-through-program-hooks.md) | Name the allocator in typy.toml and call it through program hooks | Accepted | 2026-11-17 |
| [ADR-0044](0044-run-a-librarys-module-when-it-loads-and-guard-each-export.md) | Run a library's module when it loads and guard each export | Accepted | 2026-11-17 |
| [ADR-0045](0045-name-a-librarys-files-by-its-release-as-linux-names-libraries.md) | Name a library's files by its release as Linux names libraries | Accepted | 2026-11-17 |
| [ADR-0046](0046-write-a-librarys-binding-as-a-typed-function-for-each-export.md) | Write a library's binding as a typed function for each export | Accepted | 2026-11-17 |
| [ADR-0047](0047-compile-ctypes-declarations-into-direct-calls-linked-at-build-time.md) | Compile ctypes declarations into direct calls linked at build time | Accepted | 2026-11-17 |
| [ADR-0048](0048-hold-a-ctypes-object-as-a-counted-object-over-a-c-buffer.md) | Hold a ctypes object as a counted object over a C buffer | Accepted | 2026-11-17 |
| [ADR-0049](0049-lay-out-a-ctypes-structure-as-c-and-pass-it-as-system-v-does.md) | Lay out a ctypes structure as C and pass it as System V does | Superseded by ADR-0052 | 2026-11-17 |
| [ADR-0050](0050-build-the-documentation-with-zensical-and-publish-it-on-github-pages.md) | Build the documentation with Zensical and publish it on GitHub Pages | Accepted | 2026-11-17 |
| [ADR-0051](0051-build-for-a-target-with-the-cross-toolchain-named-after-its-triple.md) | Build for a target with the cross toolchain named after its triple | Accepted | 2026-11-17 |
| [ADR-0052](0052-pass-a-ctypes-structure-by-value-as-the-targets-c-convention-does.md) | Pass a ctypes structure by value as the target's C convention does | Accepted | 2026-11-17 |
| [ADR-0053](0053-return-a-float-the-runtime-may-raise-beside-as-its-bits.md) | Return a float the runtime may raise beside as its bits | Accepted | 2026-11-17 |
| [ADR-0054](0054-build-and-pin-llvm-for-every-host-from-its-sources.md) | Build and pin LLVM for every host from its sources | Accepted | 2026-11-18 |
| [ADR-0055](0055-support-macos-on-apple-silicon-as-a-host-that-builds-for-itself.md) | Support macOS on Apple silicon as a host that builds for itself | Accepted | 2026-11-18 |
| [ADR-0056](0056-link-for-mach-o-with-apples-linker-and-name-libraries-as-macos-does.md) | Link for Mach-O with Apple's linker and name libraries as macOS does | Accepted | 2026-11-18 |
| [ADR-0057](0057-give-the-runtime-each-systems-c-constants-and-macos-its-own-error-texts.md) | Give the runtime each system's C constants and macOS its own error texts | Accepted | 2026-11-18 |
| [ADR-0058](0058-pass-c-arguments-as-libffi-passes-them-on-every-target.md) | Pass C arguments as libffi passes them on every target | Accepted | 2026-11-18 |
| [ADR-0059](0059-link-nothing-more-for-ctypes-cdll-of-none.md) | Link nothing more for `ctypes.CDLL(None)` | Accepted | 2026-11-18 |
| [ADR-0060](0060-make-a-function-value-an-object-of-its-code-and-its-captures.md) | Make a function value an object of its code and its captures | Accepted | 2026-11-18 |
| [ADR-0061](0061-capture-by-value-unless-the-variable-is-assigned-after-the-capture.md) | Capture by value unless the variable is assigned after the capture | Accepted | 2026-11-18 |
| [ADR-0062](0062-store-attributes-and-tuple-items-in-decreasing-order-of-alignment.md) | Store attributes and tuple items in decreasing order of alignment | Accepted | 2026-11-18 |
| [ADR-0063](0063-compile-a-generator-as-a-resumable-function-with-its-frame-in-its-object.md) | Compile a generator as a resumable function with its frame in its object | Accepted | 2026-11-19 |
| [ADR-0064](0064-close-a-generator-freed-while-suspended-at-its-yield.md) | Close a generator freed while suspended at its yield | Superseded by ADR-0069 | 2026-11-19 |
| [ADR-0065](0065-give-each-iterable-type-iteration-entries-the-runtime-calls.md) | Give each iterable type iteration entries the runtime calls | Accepted | 2026-11-19 |
| [ADR-0066](0066-hold-an-abstract-collection-as-the-union-of-its-built-in-types.md) | Hold an abstract collection as the union of its built-in types | Superseded by ADR-0086 | 2026-11-19 |
| [ADR-0067](0067-type-what-a-yield-receives-as-the-send-type-or-none.md) | Type what a `yield` receives as the send type or `None` | Accepted | 2026-11-19 |
| [ADR-0068](0068-give-the-caller-back-its-handled-exception-when-a-generator-step-stops.md) | Give the caller back its handled exception when a generator step stops | Accepted | 2026-11-19 |
| [ADR-0069](0069-close-freed-generators-from-a-queue-after-the-call-that-dropped-them.md) | Close freed generators from a queue after the call that dropped them | Accepted | 2026-11-19 |
| [ADR-0070](0070-keep-the-variables-of-each-function-an-exception-leaves-with-it.md) | Keep the variables of each function an exception leaves with it | Accepted | 2026-11-19 |
| [ADR-0071](0071-release-the-module-variables-when-a-program-with-generators-ends.md) | Release the module variables when a program with generators ends | Accepted | 2026-11-19 |
| [ADR-0072](0072-delegate-yield-from-one-step-at-a-time-through-the-runtime.md) | Delegate `yield from` one step at a time through the runtime | Accepted | 2026-11-19 |
| [ADR-0073](0073-bind-the-arguments-of-a-call-with-unpacking-when-the-program-runs.md) | Bind the arguments of a call with unpacking when the program runs | Accepted | 2026-11-20 |
| [ADR-0074](0074-define-a-stable-abi-for-typy-modules-at-1-0.md) | Define a stable ABI for TyPy modules at 1.0 | Accepted | 2026-11-20 |
| [ADR-0075](0075-read-the-stack-of-each-thread-a-library-runs-on-from-the-thread.md) | Read the stack of each thread a library runs on from the thread | Accepted | 2026-11-20 |
| [ADR-0076](0076-give-a-function-value-the-identity-of-the-function-it-stands-for.md) | Give a function value the identity of the function it stands for | Accepted | 2026-11-20 |
| [ADR-0077](0077-type-a-built-in-used-as-a-value-by-the-callable-it-fills.md) | Type a built-in used as a value by the `Callable` it fills | Accepted | 2026-11-20 |
| [ADR-0078](0078-make-map-and-filter-iterator-objects-of-the-runtime.md) | Make `map` and `filter` iterator objects of the runtime | Accepted | 2026-11-20 |
| [ADR-0079](0079-bind-a-method-value-to-the-method-the-receivers-own-class-runs.md) | Bind a method value to the method the receiver's own class runs | Accepted | 2026-11-20 |
| [ADR-0080](0080-call-a-function-with-defaults-or-args-through-an-adapter.md) | Call a function with defaults or `*args` through an adapter | Accepted | 2026-11-20 |
| [ADR-0081](0081-check-a-nested-function-of-a-generic-function-in-each-copy.md) | Check a nested function of a generic function in each copy | Accepted | 2026-11-20 |
| [ADR-0082](0082-bind-a-decorated-functions-name-to-what-its-decorators-return.md) | Bind a decorated function's name to what its decorators return | Accepted | 2026-11-20 |
| [ADR-0083](0083-describe-each-built-in-so-calls-with-unpacking-bind-when-they-run.md) | Describe each built-in so calls with unpacking bind when they run | Accepted | 2026-11-20 |
| [ADR-0084](0084-let-type-c-hold-c-or-a-class-derived-from-it.md) | Let `type[C]` hold `C` or a class derived from it | Accepted | 2026-11-20 |
| [ADR-0085](0085-hold-a-named-tuple-as-an-instance-of-a-class-derived-from-tuple.md) | Hold a named tuple as an instance of a class derived from `tuple` | Superseded by ADR-0088 | 2026-11-20 |
| [ADR-0086](0086-hold-an-abstract-collection-as-its-built-in-types-range-included.md) | Hold an abstract collection as its built-in types, `range` included | Accepted | 2026-11-20 |
| [ADR-0087](0087-make-typing-values-constants-written-only-where-the-oracles-agree.md) | Make typing values constants, written only where the oracles agree | Accepted | 2026-11-20 |
| [ADR-0088](0088-give-a-named-tuple-methods-over-the-tuple-of-its-fields.md) | Give a named tuple methods over the tuple of its fields | Accepted | 2026-11-21 |
