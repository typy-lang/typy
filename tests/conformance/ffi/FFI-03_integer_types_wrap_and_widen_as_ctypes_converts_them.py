# typy-test: FFI-03
# typy-expect: accepted
import ctypes

libc = ctypes.CDLL("libc.so.6")
libc.llabs.argtypes = [ctypes.c_longlong]
libc.llabs.restype = ctypes.c_longlong
libc.labs.argtypes = [ctypes.c_int8]
libc.labs.restype = ctypes.c_int8
libc.abs.argtypes = [ctypes.c_uint16]
libc.abs.restype = ctypes.c_uint16
libc.atoi.argtypes = [ctypes.c_char_p]
libc.atoi.restype = ctypes.c_uint8
libc.strtoul.argtypes = [ctypes.c_char_p, ctypes.c_void_p, ctypes.c_int]
libc.strtoul.restype = ctypes.c_uint64
libc.strtol.argtypes = [ctypes.c_char_p, ctypes.c_void_p, ctypes.c_int]
libc.strtol.restype = ctypes.c_int32

for value in [0, -1, 127, -129, 300, 2**31, 2**63, -(2**63), 2**64 + 5, -(2**70) - 3]:
    print(value, libc.llabs(value), libc.labs(value), libc.abs(value))
print(libc.llabs(True), libc.labs(False))
print(libc.atoi(b"300"), libc.atoi(b"-1"))
print(libc.strtoul(b"18446744073709551615", None, 10), libc.strtoul(b"ff", None, 16))
print(libc.strtol(b"4294967297", None, 10), libc.strtol(b"-2147483649", None, 10))
