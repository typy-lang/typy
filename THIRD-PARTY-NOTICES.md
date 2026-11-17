# Third-party notices

TyPy's own code is licensed under either of the
[Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option. Parts of the runtime come from
other projects and keep their own licences, listed below. The runtime is
linked into every program TyPy builds, so those programs contain these parts
too, and the notices below go with them.

## Libraries linked into TyPy

The release archive holds the licences of the libraries its files contain,
in its `licenses/` directory:

| File | Covers |
| --- | --- |
| `licenses/crates.html` | The Rust crates linked into `typy`, listed from `Cargo.lock` by cargo-about with `about.toml` |
| `licenses/llvm.txt` | LLVM, which `typy` links statically |
| `licenses/rust-library.html` | Rust's standard library, which `typy` links and whose `core` every program gets through the runtime |

The runtime uses no crates besides Rust's own libraries.

## CPython

Four files of the runtime are derived from the source code of CPython 3.15.
Each port keeps CPython's algorithm, constants and order of operations, so
TyPy's results agree with CPython's to the last bit, and its sort makes the
same comparisons in the same order.

| File | Derived from |
| --- | --- |
| `crates/typy-runtime/src/list/listsort.rs` | `listsort` in `Objects/listobject.c`: `count_run`, `minrun_next`, binary insertion, the powersort merge strategy, `merge_at`, `merge_lo`, `merge_hi`, `gallop_left` and `gallop_right` |
| `crates/typy-runtime/src/math/special.rs` | `pow`, `fmod`, `remainder`, `ldexp`, `ulp`, `nextafter`, `fma`, `isclose` and `hypot` with `vector_norm` in `Modules/mathmodule.c` |
| `crates/typy-runtime/src/math/gamma.rs` | `m_tgamma`, `m_lgamma`, `m_sinpi`, `lanczos_sum` and the Lanczos coefficients in `Modules/mathmodule.c` |
| `crates/typy-runtime/src/math/sums.rs` | `fsum`, `dist`, `prod` and `sumprod` with its triple-length arithmetic in `Modules/mathmodule.c` |

Summary of the changes made to Python: each part is rewritten from C in Rust.
It works on TyPy's own representation of integers, floats and lists instead
of CPython's objects, calls the comparison TyPy's runtime gives instead of
`PyObject_RichCompare`, and reports errors by raising TyPy's exceptions with
CPython's messages. Nothing else of CPython is included.

Copyright (c) 2001 Python Software Foundation; All Rights Reserved

```text
PYTHON SOFTWARE FOUNDATION LICENSE VERSION 2
--------------------------------------------

1. This LICENSE AGREEMENT is between the Python Software Foundation
("PSF"), and the Individual or Organization ("Licensee") accessing and
otherwise using this software ("Python") in source or binary form and
its associated documentation.

2. Subject to the terms and conditions of this License Agreement, PSF hereby
grants Licensee a nonexclusive, royalty-free, world-wide license to reproduce,
analyze, test, perform and/or display publicly, prepare derivative works,
distribute, and otherwise use Python alone or in any derivative version,
provided, however, that PSF's License Agreement and PSF's notice of copyright,
i.e., "Copyright (c) 2001 Python Software Foundation; All Rights Reserved"
are retained in Python alone or in any derivative version prepared by Licensee.

3. In the event Licensee prepares a derivative work that is based on
or incorporates Python or any part thereof, and wants to make
the derivative work available to others as provided herein, then
Licensee hereby agrees to include in any such work a brief summary of
the changes made to Python.

4. PSF is making Python available to Licensee on an "AS IS"
basis.  PSF MAKES NO REPRESENTATIONS OR WARRANTIES, EXPRESS OR
IMPLIED.  BY WAY OF EXAMPLE, BUT NOT LIMITATION, PSF MAKES NO AND
DISCLAIMS ANY REPRESENTATION OR WARRANTY OF MERCHANTABILITY OR FITNESS
FOR ANY PARTICULAR PURPOSE OR THAT THE USE OF PYTHON WILL NOT
INFRINGE ANY THIRD PARTY RIGHTS.

5. PSF SHALL NOT BE LIABLE TO LICENSEE OR ANY OTHER USERS OF PYTHON
FOR ANY INCIDENTAL, SPECIAL, OR CONSEQUENTIAL DAMAGES OR LOSS AS
A RESULT OF MODIFYING, DISTRIBUTING, OR OTHERWISE USING PYTHON,
OR ANY DERIVATIVE THEREOF, EVEN IF ADVISED OF THE POSSIBILITY THEREOF.

6. This License Agreement will automatically terminate upon a material
breach of its terms and conditions.

7. Nothing in this License Agreement shall be deemed to create any
relationship of agency, partnership, or joint venture between PSF and
Licensee.  This License Agreement does not grant permission to use PSF
trademarks or trade name in a trademark sense to endorse or promote
products or services of Licensee, or any third party.

8. By copying, installing or otherwise using Python, Licensee
agrees to be bound by the terms and conditions of this License
Agreement.
```

## Unicode Character Database

Two files of the runtime hold tables generated from the Unicode Character
Database by `scripts/generate-unicode-tables.py`, from the version CPython's
`unicodedata` module provides, which the first lines of each file name:
`crates/typy-runtime/src/string/unicode/tables.rs` and
`crates/typy-runtime/src/string/unicode/case_tables.rs`.

```text
UNICODE LICENSE V3

COPYRIGHT AND PERMISSION NOTICE

Copyright (c) 1991-2026 Unicode, Inc.

NOTICE TO USER: Carefully read the following legal agreement. BY
DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.

Permission is hereby granted, free of charge, to any person obtaining a
copy of data files and any associated documentation (the "Data Files") or
software and any associated documentation (the "Software") to deal in the
Data Files or Software without restriction, including without limitation
the rights to use, copy, modify, merge, publish, distribute, and/or sell
copies of the Data Files or Software, and to permit persons to whom the
Data Files or Software are furnished to do so, provided that either (a)
this copyright and permission notice appear with all copies of the Data
Files or Software, or (b) this copyright and permission notice appear in
associated Documentation.

THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
THIRD PARTY RIGHTS.

IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
FILES OR SOFTWARE.

Except as contained in this notice, the name of a copyright holder shall
not be used in advertising or otherwise to promote the sale, use or other
dealings in these Data Files or Software without prior written
authorization of the copyright holder.
```
