# Modules

A TyPy 0.2 program is one module, its main file, which runs as
`__main__` (STD-06). Imports of TyPy modules and of the standard library
come in 0.3 (SMT-07, MOD-01 to MOD-05), and lazy imports for programs whose
floor is 3.15 in 0.3 (MOD-06 to MOD-09).

## What TyPy 0.2 accepts

<!-- typy-expect: accepted -->
```python
def main() -> None:
    print("running as", __name__)


if __name__ == "__main__":
    main()
```

## Planned

An import is refused until 0.3:

<!-- typy-expect: rejected E0201 at 1:1 -->
```python
import math
```
