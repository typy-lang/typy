# typy-test: FFI-02
# typy-expect: accepted
import ctypes

libc = ctypes.CDLL("libc.so.6")


def later(text: bytes) -> int:
    return libc.strlen(text)


libc.strlen.argtypes = [ctypes.c_char_p]
libc.strlen.restype = ctypes.c_size_t
libc.free.argtypes = [ctypes.c_void_p]
libc.free.restype = None

print(later(b"declared after the function, before the call"))
print(libc.free(None))
