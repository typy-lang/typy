# Security policy

## Supported versions

TyPy has not made a release yet. Once it does, the latest minor release
receives security fixes.

| Version | Supported |
| --- | --- |
| Unreleased `main` | Yes |

## Reporting a vulnerability

Report a vulnerability privately through the repository's security
advisory form at
<https://github.com/typy-lang/typy/security/advisories/new>, or open the
Security tab of <https://github.com/typy-lang/typy> and choose "Report a
vulnerability".
Do not open a public issue. You will get an answer within seven days, and a
fix or a plan with a date within 30 days.

## Design notes that matter for security

- The compiler runs no code from the program it compiles: there are no
  build scripts, no macros and no `eval`.
- A source file larger than 16 MiB is refused before it is read (LEX-09).
- The linker runs as a subprocess with an argument list and no shell.
- The compiler makes no network access (C-05).
- Programs built by TyPy check every index, use integers that do not
  overflow, and free memory exactly once. Calls into C through `ctypes` are
  outside these guarantees, as for any C code.
- The full threat model is section 7.7 of the
  [architecture](docs/architecture/architecture.md).
