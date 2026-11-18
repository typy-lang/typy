//! End-to-end tests of C functions that take and return `ctypes` structures by value, and
//! take more narrow integers than registers, as each target's C calling convention passes
//! them: the C library is built as a developer builds it, and CPython and the TyPy program
//! call it with the same source; the program is also built for the other architecture and run
//! under its emulator (FFI-03, FFI-12, FFI-13, ADR-0052, ADR-0058).

use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};

/// Functions of structures that pass in general registers, vector registers, both, and
/// memory, of calls whose registers run out before a structure, of four doubles, which
/// AArch64 passes in registers, and of five floats, which it passes as a copy the callee may
/// change; and of narrow integers past the registers, after a structure that no longer fits in
/// them, which Apple's arm64 places on the stack at their own sizes. `c_byte` is C's
/// `signed char`: plain `char` is unsigned on AArch64.
const SHAPES: &str = r"
typedef struct { int a; double b; } Mixed;
typedef struct { float x, y, z; } Vec3;
typedef struct { double x, y; } Pair;
typedef struct { long a, b, c; } Triple;
typedef struct { signed char c; short s; } Small;
typedef struct { signed char a, b, c; } Odd;
typedef struct { double a, b, c, d; } Quad;
typedef struct { float a, b, c, d, e; } Five;

Mixed mixed_scale(Mixed m, int k) { Mixed r = { m.a * k, m.b * k }; return r; }
Vec3 vec3_add(Vec3 a, Vec3 b) { Vec3 r = { a.x + b.x, a.y + b.y, a.z + b.z }; return r; }
double pair_sum(Pair p) { return p.x + p.y; }
Pair pair_swap(Pair p) { Pair r = { p.y, p.x }; return r; }
Triple triple_shift(Triple t, long d) { Triple r = { t.a + d, t.b + d, t.c + d }; return r; }
long triple_sum(Triple t) { return t.a + t.b + t.c; }
int small_sum(Small s) { return s.c + s.s; }
Odd odd_next(Odd o) { Odd r = { o.a + 1, o.b + 1, o.c + 1 }; return r; }
long crowd(long a, long b, long c, long d, long e, long f, Mixed m) {
    return a + b + c + d + e + f + m.a + (long) m.b;
}
double floats(double a, double b, double c, double d, double e, double f, double g, Pair p) {
    return a + b + c + d + e + f + g + p.x * 10 + p.y * 100;
}
Quad quad_scale(Quad q, double k) { Quad r = { q.a * k, q.b * k, q.c * k, q.d * k }; return r; }
double quad_sum(Quad q) { return q.a + q.b + q.c + q.d; }
Five five_add(Five x, Five y) {
    Five r = { x.a + y.a, x.b + y.b, x.c + y.c, x.d + y.d, x.e + y.e };
    x.a = 99;
    return r;
}
long narrow(signed char a, unsigned char b, short c, unsigned short d, int e, unsigned int f,
            _Bool g, Mixed m, signed char h, unsigned char i, short j, unsigned short k,
            _Bool l) {
    return a + 2 * b + 3 * c + 4 * d + 5L * e + 6L * f + 7 * g + 8L * m.a + (long) m.b
        + 9 * h + 10 * i + 11 * j + 12 * k + 13 * l;
}
";

/// A program that declares and calls each function of `SHAPES`.
const PROGRAM: &str = r#"
import ctypes

lib = ctypes.CDLL("libshapes.so")


class Mixed(ctypes.Structure):
    _fields_ = [("a", ctypes.c_int), ("b", ctypes.c_double)]


class Vec3(ctypes.Structure):
    _fields_ = [("x", ctypes.c_float), ("y", ctypes.c_float), ("z", ctypes.c_float)]


class Pair(ctypes.Structure):
    _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double)]


class Triple(ctypes.Structure):
    _fields_ = [("a", ctypes.c_long), ("b", ctypes.c_long), ("c", ctypes.c_long)]


class Small(ctypes.Structure):
    _fields_ = [("c", ctypes.c_byte), ("s", ctypes.c_short)]


class Odd(ctypes.Structure):
    _fields_ = [("a", ctypes.c_byte), ("b", ctypes.c_byte), ("c", ctypes.c_byte)]


class Quad(ctypes.Structure):
    _fields_ = [
        ("a", ctypes.c_double),
        ("b", ctypes.c_double),
        ("c", ctypes.c_double),
        ("d", ctypes.c_double),
    ]


class Five(ctypes.Structure):
    _fields_ = [
        ("a", ctypes.c_float),
        ("b", ctypes.c_float),
        ("c", ctypes.c_float),
        ("d", ctypes.c_float),
        ("e", ctypes.c_float),
    ]


lib.mixed_scale.argtypes = [Mixed, ctypes.c_int]
lib.mixed_scale.restype = Mixed
lib.vec3_add.argtypes = [Vec3, Vec3]
lib.vec3_add.restype = Vec3
lib.pair_sum.argtypes = [Pair]
lib.pair_sum.restype = ctypes.c_double
lib.pair_swap.argtypes = [Pair]
lib.pair_swap.restype = Pair
lib.triple_shift.argtypes = [Triple, ctypes.c_long]
lib.triple_shift.restype = Triple
lib.triple_sum.argtypes = [Triple]
lib.triple_sum.restype = ctypes.c_long
lib.small_sum.argtypes = [Small]
lib.small_sum.restype = ctypes.c_int
lib.odd_next.argtypes = [Odd]
lib.odd_next.restype = Odd
L = ctypes.c_long
lib.crowd.argtypes = [L, L, L, L, L, L, Mixed]
lib.crowd.restype = ctypes.c_long
D = ctypes.c_double
lib.floats.argtypes = [D, D, D, D, D, D, D, Pair]
lib.floats.restype = ctypes.c_double
lib.quad_scale.argtypes = [Quad, D]
lib.quad_scale.restype = Quad
lib.quad_sum.argtypes = [Quad]
lib.quad_sum.restype = D
lib.five_add.argtypes = [Five, Five]
lib.five_add.restype = Five
B = ctypes.c_byte
UB = ctypes.c_ubyte
S = ctypes.c_short
US = ctypes.c_ushort
lib.narrow.argtypes = [
    B, UB, S, US, ctypes.c_int, ctypes.c_uint, ctypes.c_bool, Mixed, B, UB, S, US, ctypes.c_bool
]
lib.narrow.restype = ctypes.c_long

m = lib.mixed_scale(Mixed(3, 1.5), 4)
print(m.a, m.b)
v = lib.vec3_add(Vec3(1, 2, 3), Vec3(0.5, 0.25, 0.125))
print(v.x, v.y, v.z)
print(lib.pair_sum(Pair(1.5, 2.25)))
s = lib.pair_swap(Pair(1.5, 2.25))
print(s.x, s.y)
t = lib.triple_shift(Triple(1, 2, 3), 10)
print(t.a, t.b, t.c, lib.triple_sum(t))
print(lib.small_sum(Small(-3, 300)))
o = lib.odd_next(Odd(1, 2, 3))
print(o.a, o.b, o.c)
print(lib.crowd(1, 2, 3, 4, 5, 6, Mixed(7, 8.5)))
print(lib.floats(1, 2, 3, 4, 5, 6, 7, Pair(1, 2)))
q = lib.quad_scale(Quad(1, 2, 3, 4), 0.5)
print(q.a, q.b, q.c, q.d, lib.quad_sum(q))
f = Five(1, 2, 3, 4, 5)
g = lib.five_add(f, Five(0.5, 0.5, 0.5, 0.5, 0.5))
print(g.a, g.b, g.c, g.d, g.e, f.a)
m = Mixed(5, 2.5)
print(lib.narrow(-1, 255, -300, 65535, -70000, 4000000000, True, m, -128, 200, 1000, 60000, True))
"#;

/// What CPython prints for `PROGRAM`.
const EXPECTED: &str = "12 6.0\n\
1.5 2.25 3.125\n\
3.75\n\
2.25 1.5\n\
11 12 13 36\n\
297\n\
2 3 4\n\
36\n\
238.0\n\
0.5 1.0 1.5 2.0 5.0\n\
1.5 2.5 3.5 4.5 5.5 1.0\n\
24000643659\n\
";

/// How this system builds and finds a shared C library: its file, the C compiler's flags that
/// make it, and the variable the dynamic loader searches.
struct SharedLibrary {
    file: &'static str,
    flags: &'static [&'static str],
    search: &'static str,
}

#[cfg(target_os = "macos")]
const SHARED: SharedLibrary = SharedLibrary {
    file: "libshapes.dylib",
    flags: &["-dynamiclib", "-Wl,-install_name,libshapes.dylib"],
    search: "DYLD_LIBRARY_PATH",
};

#[cfg(not(target_os = "macos"))]
const SHARED: SharedLibrary = SharedLibrary {
    file: "libshapes.so",
    flags: &["-shared", "-fPIC"],
    search: "LD_LIBRARY_PATH",
};

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A directory with the C library's source and the program, set to find the library in `lib`.
fn project() -> (tempfile::TempDir, Utf8PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    std::fs::create_dir(root.join("lib")).unwrap();
    std::fs::write(root.join("shapes.c"), SHAPES).unwrap();
    std::fs::write(
        root.join("app.py"),
        PROGRAM.replace("libshapes.so", SHARED.file),
    )
    .unwrap();
    std::fs::write(root.join("typy.toml"), "library-paths = [\"lib\"]\n").unwrap();
    (directory, root)
}

/// Builds the C library with the C compiler `cc` and the program with `typy build` and
/// `arguments`.
fn build(root: &Utf8Path, cc: &str, arguments: &[&str]) {
    let compiled = Command::new(cc)
        .args(SHARED.flags)
        .args(["-O1", "-o", &format!("lib/{}", SHARED.file), "shapes.c"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(compiled.status.success(), "{}", text(&compiled.stderr));
    let built = Command::new(env!("CARGO_BIN_EXE_typy"))
        .args(["build", "app.py", "-o", "app"])
        .args(arguments)
        .current_dir(root)
        .env("TYPY_HOME", root.join("home"))
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", text(&built.stderr));
}

#[test]
fn ffi_03_structures_pass_by_value_as_cpython_passes_them() {
    let (_directory, root) = project();
    build(&root, "cc", &[]);
    let oracle = Command::new("python3")
        .args(["-I", "app.py"])
        .current_dir(&root)
        .env(SHARED.search, root.join("lib"))
        .output()
        .unwrap();
    assert_eq!(text(&oracle.stdout), EXPECTED, "{}", text(&oracle.stderr));
    let run = Command::new(root.join("app"))
        .env(SHARED.search, root.join("lib"))
        .output()
        .unwrap();
    assert_eq!(text(&run.stdout), EXPECTED, "{}", text(&run.stderr));
}

#[test]
#[cfg(target_os = "linux")]
fn ffi_12_structures_pass_by_value_on_the_other_architecture() {
    let (triple, gnu, emulator) = if cfg!(target_arch = "aarch64") {
        (
            "x86_64-unknown-linux-gnu",
            "x86_64-linux-gnu",
            "qemu-x86_64",
        )
    } else {
        (
            "aarch64-unknown-linux-gnu",
            "aarch64-linux-gnu",
            "qemu-aarch64",
        )
    };
    let (_directory, root) = project();
    build(&root, &format!("{gnu}-gcc"), &["--target", triple]);
    let run = Command::new(emulator)
        .args(["-L", &format!("/usr/{gnu}")])
        .args(["-E", &format!("{}={}", SHARED.search, root.join("lib"))])
        .arg(root.join("app"))
        .output()
        .unwrap();
    assert_eq!(text(&run.stdout), EXPECTED, "{}", text(&run.stderr));
}
