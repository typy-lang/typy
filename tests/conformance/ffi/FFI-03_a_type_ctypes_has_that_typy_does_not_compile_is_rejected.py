# typy-test: FFI-03
# typy-expect: rejected E0430 at 6:26
import ctypes

libc = ctypes.CDLL("libc.so.6")
libc.toupper.argtypes = [ctypes.c_char]
