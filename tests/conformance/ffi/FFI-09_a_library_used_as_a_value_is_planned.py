# typy-test: FFI-09
# typy-expect: rejected E0201 at 6:7
import ctypes

libc = ctypes.CDLL("libc.so.6")
print(libc)
