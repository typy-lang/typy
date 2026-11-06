# TyPy documentation

TyPy is an independent implementation of Python that compiles a strict
subset of standard Python ahead of time to native code through LLVM. The
engineering documents below are written in order, and each builds on the one
before.

| Order | Document | Status |
| --- | --- | --- |
| 1 | [Research](research/research.md) | Approved |
| 2 | [Software requirements specification](requirements/srs.md) | Approved |
| 3 | [Use case model](modeling/use-case-model.md) | Approved |
| 4 | [Domain model](modeling/domain-model.md) | Approved |
| 5 | [Software architecture](architecture/architecture.md) | Approved |
| 6 | [Architecture decision records](adr/index.md) | Seven records, Accepted |
| 7 | [Detailed design](design/detailed-design.md) | Draft |

## Diagrams

| Diagram | UML type | Used by |
| --- | --- | --- |
| [TyPy use cases and their actors](diagrams/use-case-typy.svg) | Use case diagram | Use case model |
| [How TyPy use cases build on each other](diagrams/use-case-relationships.svg) | Use case diagram | Use case model |
| [TyPy bounded contexts](diagrams/package-context-map.svg) | Package diagram | Domain model |
| [Source and checking concepts](diagrams/class-domain-source-and-checking.svg) | Class diagram, conceptual | Domain model |
| [Lowering, code generation and build concepts](diagrams/class-domain-lowering-and-build.svg) | Class diagram, conceptual | Domain model |
| [Runtime and conformance concepts](diagrams/class-domain-runtime.svg) | Class diagram, conceptual | Domain model |
| [Lifecycle of a heap object](diagrams/state-heap-object.svg) | State machine diagram | Domain model |
| [Lifecycle of an exception](diagrams/state-exception.svg) | State machine diagram | Domain model |
| [Lifecycle of a module in the compiler](diagrams/state-module.svg) | State machine diagram | Domain model |
| [TyPy in its environment](diagrams/component-system-context.svg) | Component diagram, system context | Software architecture |
| [TyPy building blocks](diagrams/component-building-blocks.svg) | Component diagram | Software architecture |
| [Building an executable](diagrams/sequence-build-executable.svg) | Sequence diagram | Software architecture |
| [An exception returned through two frames](diagrams/sequence-raise-exception.svg) | Sequence diagram | Software architecture |
| [Running one conformance test](diagrams/sequence-conformance-test.svg) | Sequence diagram | Software architecture |
| [Collecting reference cycles](diagrams/activity-cycle-collection.svg) | Activity diagram | Software architecture |
| [Where TyPy and its output run](diagrams/deployment-typy.svg) | Deployment diagram | Software architecture |
| [Design classes of typy_diagnostics and typy_source](diagrams/class-design-source.svg) | Class diagram, design | Detailed design |
| [Design classes of typy_check](diagrams/class-design-check.svg) | Class diagram, design | Detailed design |
| [Design classes of typy_tyir and typy_lower](diagrams/class-design-tyir-and-lowering.svg) | Class diagram, design | Detailed design |
| [Design classes of typy_codegen and typy_build](diagrams/class-design-codegen-and-build.svg) | Class diagram, design | Detailed design |
| [Design classes of the runtime](diagrams/class-design-runtime.svg) | Class diagram, design | Detailed design |

Each diagram is a PlantUML source in `docs/diagrams/`, rendered to the SVG
beside it by `make diagrams`, which runs PlantUML in the `diagrams`
container.
