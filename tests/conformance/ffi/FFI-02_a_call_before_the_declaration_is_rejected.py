# typy-test: FFI-02
# typy-expect: rejected E0430 at 6:7
import ctypes

libc = ctypes.CDLL("libc.so.6")
print(libc.abs(-3))
libc.abs.argtypes = [ctypes.c_int]
libc.abs.restype = ctypes.c_int
