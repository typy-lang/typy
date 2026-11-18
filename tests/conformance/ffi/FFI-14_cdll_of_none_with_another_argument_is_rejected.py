# typy-test: FFI-14
# typy-expect: rejected E0430 at 5:7
import ctypes

own = ctypes.CDLL(None, 0)
own.labs.argtypes = [ctypes.c_long]
own.labs.restype = ctypes.c_long
print(own.labs(-1))
