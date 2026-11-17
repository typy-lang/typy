//! End-to-end tests of C functions that take and return `ctypes` structures by value, as
//! the System V calling convention of x86-64 Linux passes them: the C library is built as a
//! developer builds it, and CPython and the TyPy program call it with the same source
//! (FFI-03, ADR-0049).

use std::process::Command;

use camino::Utf8PathBuf;

/// Functions of structures that pass in general registers, vector registers, both, and
/// memory, and of calls whose registers run out before a structure.
const SHAPES: &str = r"
typedef struct { int a; double b; } Mixed;
typedef struct { float x, y, z; } Vec3;
typedef struct { double x, y; } Pair;
typedef struct { long a, b, c; } Triple;
typedef struct { char c; short s; } Small;
typedef struct { char a, b, c; } Odd;

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
";

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn ffi_03_structures_pass_by_value_as_cpython_passes_them() {
    let directory = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap();
    std::fs::create_dir(root.join("lib")).unwrap();
    std::fs::write(root.join("shapes.c"), SHAPES).unwrap();
    std::fs::write(root.join("app.py"), PROGRAM).unwrap();
    std::fs::write(root.join("typy.toml"), "library-paths = [\"lib\"]\n").unwrap();
    let compiled = Command::new("cc")
        .args([
            "-shared",
            "-fPIC",
            "-O1",
            "-o",
            "lib/libshapes.so",
            "shapes.c",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(compiled.status.success(), "{}", text(&compiled.stderr));
    let oracle = Command::new("python3")
        .args(["-I", "app.py"])
        .current_dir(&root)
        .env("LD_LIBRARY_PATH", root.join("lib"))
        .output()
        .unwrap();
    assert_eq!(text(&oracle.stdout), EXPECTED, "{}", text(&oracle.stderr));
    let built = Command::new(env!("CARGO_BIN_EXE_typy"))
        .args(["build", "app.py", "-o", "app"])
        .current_dir(&root)
        .env("TYPY_HOME", root.join("home"))
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", text(&built.stderr));
    let run = Command::new(root.join("app"))
        .env("LD_LIBRARY_PATH", root.join("lib"))
        .output()
        .unwrap();
    assert_eq!(text(&run.stdout), EXPECTED, "{}", text(&run.stderr));
}
