# typy-test: FFI-02
# typy-expect: rejected E0430 at 7:7
import ctypes

libc = ctypes.CDLL("libc.so.6")
libc.abs.argtypes = [ctypes.c_int]
print(libc.abs(-3))
