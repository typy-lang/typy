# typy-test: FFI-01
# typy-expect: rejected E0430 at 6:8
import ctypes

name = "libc.so.6"
libc = ctypes.CDLL(name)
