# C interoperability

Calling C libraries through the standard `ctypes` module, and exporting
functions to C, come in 0.5 (FFI-01 to FFI-05). TyPy 0.2 programs run on
their own, and a built executable needs no Python installation.

## What TyPy 0.2 accepts

A program built by `typy build` is a native executable linked against the
C library, with no interpreter:

<!-- typy-expect: accepted -->
```python
print("a native executable says hello")
```

## Planned

`ctypes` is refused until 0.5, as every import is until 0.3:

<!-- typy-expect: rejected E0201 at 1:1 -->
```python
import ctypes
```
