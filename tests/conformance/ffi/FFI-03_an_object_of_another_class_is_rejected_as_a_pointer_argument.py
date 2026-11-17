# typy-test: FFI-03
# typy-expect: rejected E0404 at 8:23
import ctypes

libm = ctypes.CDLL("libm.so.6")
libm.frexp.argtypes = [ctypes.c_double, ctypes.POINTER(ctypes.c_int)]
libm.frexp.restype = ctypes.c_double
print(libm.frexp(1.0, ctypes.c_long(1)))
