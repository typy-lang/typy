# typy-test: FFI-03
# typy-expect: accepted
import ctypes

libm = ctypes.CDLL(None)
libc = ctypes.CDLL(None)
libm.cbrtf.argtypes = [ctypes.c_float]
libm.cbrtf.restype = ctypes.c_float
libm.fabs.argtypes = [ctypes.c_double]
libm.fabs.restype = ctypes.c_double
libc.isdigit.argtypes = [ctypes.c_int]
libc.isdigit.restype = ctypes.c_bool
libc.abs.argtypes = [ctypes.c_bool]
libc.abs.restype = ctypes.c_int
libc.strchr.argtypes = [ctypes.c_char_p, ctypes.c_int]
libc.strchr.restype = ctypes.c_char_p
libc.strlen.argtypes = [ctypes.c_char_p]
libc.strlen.restype = ctypes.c_size_t
libc.malloc.argtypes = [ctypes.c_size_t]
libc.malloc.restype = ctypes.c_void_p
libc.free.argtypes = [ctypes.c_void_p]
libc.free.restype = None


def length(text: bytes | None) -> int:
    if text is None:
        return -1
    return libc.strlen(text)


print(libm.cbrtf(27.0), libm.cbrtf(0.1), libm.cbrtf(2), libm.fabs(-2), libm.fabs(True))
print(libc.isdigit(ord("7")), libc.isdigit(ord("x")))
print(libc.abs(5), libc.abs(0), libc.abs(""), libc.abs("x"), libc.abs([1]), libc.abs(2.5))
print(libc.strchr(b"hello", ord("l")), libc.strchr(b"hello", ord("z")), length(b"abc"), length(None))
block = libc.malloc(32)
print(block is not None, libc.free(block), libc.free(None))
