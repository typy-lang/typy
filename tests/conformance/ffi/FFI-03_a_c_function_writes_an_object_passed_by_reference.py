# typy-test: FFI-03
# typy-expect: accepted
import ctypes

libm = ctypes.CDLL("libm.so.6")
libc = ctypes.CDLL("libc.so.6")
IntPointer = ctypes.POINTER(ctypes.c_int)
libm.frexp.argtypes = [ctypes.c_double, IntPointer]
libm.frexp.restype = ctypes.c_double
libm.modf.argtypes = [ctypes.c_double, ctypes.POINTER(ctypes.c_double)]
libm.modf.restype = ctypes.c_double
libc.memset.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_size_t]
libc.memset.restype = None

exponent = ctypes.c_int()
print(libm.frexp(48.0, ctypes.byref(exponent)), exponent.value)

# An object passed where a pointer is declared goes by reference.
print(libm.frexp(0.375, exponent), exponent)

whole = ctypes.c_double()
print(libm.modf(3.25, whole), whole.value)

# An array passes the address of its first item.
exponents = (ctypes.c_int * 2)(5, 6)
print(libm.frexp(8.0, exponents), list(exponents))

pointer = ctypes.pointer(exponent)
print(libm.frexp(1024.0, pointer), pointer[0])

# The C function writes through `void *` too.
libc.memset(exponents, 0, ctypes.sizeof(exponents))
libc.memset(ctypes.byref(whole), 0, ctypes.sizeof(ctypes.c_double))
print(list(exponents), whole.value)


def clear(data: bytes | None) -> None:
    libc.memset(data, 0, 0)


clear(b"abc")
clear(None)


def split(value: float) -> tuple[float, int]:
    found = ctypes.c_int()
    fraction = libm.frexp(value, found)
    return fraction, found.value


print(split(-3.0))
