# typy-test: FFI-03
# typy-expect: accepted
import ctypes

libc = ctypes.CDLL(None)


class Div(ctypes.Structure):
    _fields_ = [("quot", ctypes.c_int), ("rem", ctypes.c_int)]


class LongDiv(ctypes.Structure):
    _fields_ = [("quot", ctypes.c_longlong), ("rem", ctypes.c_longlong)]


class Address(ctypes.Structure):
    _fields_ = [("s_addr", ctypes.c_uint32)]


class Pair(ctypes.Structure):
    _fields_ = [("left", ctypes.c_int), ("right", ctypes.c_int)]


libc.div.argtypes = [ctypes.c_int, ctypes.c_int]
libc.div.restype = Div
libc.lldiv.argtypes = [ctypes.c_longlong, ctypes.c_longlong]
libc.lldiv.restype = LongDiv
libc.inet_ntoa.argtypes = [Address]
libc.inet_ntoa.restype = ctypes.c_char_p
libc.memcpy.argtypes = [ctypes.POINTER(Pair), ctypes.POINTER(Pair), ctypes.c_size_t]
libc.memcpy.restype = None

result = libc.div(17, 5)
print(result.quot, result.rem)
wide = libc.lldiv(-(2**40) - 3, 7)
print(wide.quot, wide.rem)
print(libc.inet_ntoa(Address(0x0100007F)))

target = Pair()
source = Pair(3, 4)
libc.memcpy(target, ctypes.byref(source), ctypes.sizeof(Pair))
print(target.left, target.right)
