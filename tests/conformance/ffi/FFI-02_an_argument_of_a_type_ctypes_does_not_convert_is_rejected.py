# typy-test: FFI-02
# typy-expect: rejected E0404 at 8:19
import ctypes

libc = ctypes.CDLL("libc.so.6")
libc.strlen.argtypes = [ctypes.c_char_p]
libc.strlen.restype = ctypes.c_size_t
print(libc.strlen("text"))
