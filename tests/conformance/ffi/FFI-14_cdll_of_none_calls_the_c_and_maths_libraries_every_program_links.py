# typy-test: FFI-14
# typy-expect: accepted
import ctypes
from ctypes import CDLL, cdll

own = CDLL(None)
same = cdll.LoadLibrary(None)
own.labs.argtypes = [ctypes.c_long]
own.labs.restype = ctypes.c_long
own.strlen.argtypes = [ctypes.c_char_p]
own.strlen.restype = ctypes.c_size_t
same.floor.argtypes = [ctypes.c_double]
same.floor.restype = ctypes.c_double
same.fabs.argtypes = [ctypes.c_double]
same.fabs.restype = ctypes.c_double


def measure(text: bytes) -> int:
    return own.strlen(text)


print(own.labs(-12), measure(b"portable"), same.floor(-2.5), same.fabs(-0.25))
