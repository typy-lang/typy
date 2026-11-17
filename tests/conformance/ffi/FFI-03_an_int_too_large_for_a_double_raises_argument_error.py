# typy-test: FFI-03
# typy-expect: accepted
import ctypes

libm = ctypes.CDLL("libm.so.6")
libm.fabs.argtypes = [ctypes.c_double]
libm.fabs.restype = ctypes.c_double
try:
    libm.fabs(10**400)
except Exception as error:
    print(type(error).__name__, error)
print(libm.fabs(-(10**300)))
libm.fabs(-(10**309))
