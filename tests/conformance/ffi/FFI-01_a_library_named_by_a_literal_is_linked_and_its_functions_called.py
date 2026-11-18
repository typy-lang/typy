# typy-test: FFI-01
# typy-expect: accepted
# typy-system: linux
import ctypes
import ctypes as C
from ctypes import CDLL, c_double, cdll

libm = ctypes.CDLL("libm.so.6")
libc = cdll.LoadLibrary("libc.so.6")
same = CDLL("libm.so.6")
also = C.cdll.LoadLibrary("libc.so.6")

libm.cbrt.argtypes = [c_double]
libm.cbrt.restype = c_double
libc.abs.argtypes = (ctypes.c_int,)
libc.abs.restype = ctypes.c_int
same.floor.argtypes = [C.c_double]
same.floor.restype = C.c_double
also.labs.argtypes = [ctypes.c_long]
also.labs.restype = ctypes.c_long


def cube_root(value: float) -> float:
    return libm.cbrt(value)


print(cube_root(27.0), libm.cbrt(64.0), libc.abs(-7), same.floor(2.5), also.labs(-9))
