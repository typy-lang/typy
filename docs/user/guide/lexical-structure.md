# Lexical structure

TyPy reads source as the Python Language Reference defines it.

## What TyPy 0.2 accepts

| Item | Rule |
| --- | --- |
| UTF-8 source files with the `.py` extension | LEX-01 |
| Comments, line joining and indentation as in Python | LEX-02 |
| Integer literals in decimal, `0x`, `0o` and `0b` forms, with `_` separators | LEX-03 |
| Every form of floating-point literal | LEX-04 |
| String literals with every quote style, the `r` prefix and concatenation | LEX-05 |
| Bytes literals, such as `b"\x00\xff"` | LEX-06 |
| f-strings with `!s`, `!r` and `!a`, `{x = }`, nested fields and every format specification of `int`, `float`, `bool` and `str` | LEX-07 |
| A source file over 16 MiB is refused before it is read | LEX-09 |

Imaginary literals such as `2j` are refused, because TyPy has no `complex`
type (LEX-08).

## Example

<!-- typy-expect: accepted -->
```python
mask = 0b1010_1010
big = 1_000_000
ratio = 6.02e23
raw = r"C:\temp"
data = b"\x00\x7f"
name = "TyPy"
print(f"{name!r:>8} {mask:#x} {big:,} {ratio:.3e} {raw} {data} {len(data)}")
print(f"{big = }", f"{3.14159:{8}.{2}f}|", "con" "cat")
```

<!-- typy-expect: rejected E0104 at 1:5 -->
```python
z = 2j
```

An escape of a surrogate code point, `\ud800` to `\udfff` or
`\U0000d800` to `\U0000dfff`, in a `str` literal is E0201: TyPy 0.4
keeps the code point, as CPython does (LEX-10). A string that holds a
surrogate can still be made by decoding, as
`b"\xff".decode("utf-8", "surrogateescape")` gives `'\udcff'`.

<!-- typy-expect: rejected E0201 at 1:12 -->
```python
escaped = "\udcff"
```

Nesting has the limits CPython has, so a program CPython refuses is
refused with E0103 and the same message: brackets nested more than 200
deep are "too many nested parentheses", and blocks nested more than 99
deep are "too many levels of indentation". Expressions and statements
nested more than 25000 deep, past any program CPython compiles, are E0103
as well (LEX-11).
