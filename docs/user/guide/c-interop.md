# C interoperability

Calling C libraries through the standard `ctypes` module, and exporting
functions to C, come in 0.5 (FFI-01 to FFI-05). TyPy 0.4 programs run on
their own, and a built executable needs no Python installation.

## What TyPy accepts

A program built by `typy build` is a native executable linked against the
C library, with no interpreter:

<!-- typy-expect: accepted -->
```python
print("a native executable says hello")
```

## Planned

`ctypes` is refused until 0.5. In 0.5, `typy build --lib` also builds a
module as a static or shared C library: the functions its `__all__` lists
are exported under their own names, with the C types their annotations
give, and the build writes a C header, a `.pyi` stub and a `ctypes`
module that loads the library, so C programs and Python programs call it
alike (ADR-0032, FFI-05, FFI-08):

<!-- typy-expect: rejected E0201 at 1:1 -->
```python
import ctypes
```
