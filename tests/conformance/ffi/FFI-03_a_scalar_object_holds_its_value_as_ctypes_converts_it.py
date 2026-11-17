# typy-test: FFI-03
# typy-expect: accepted
import ctypes
from ctypes import c_int

number = c_int(5)
print(number, number.value, bool(number), bool(c_int()))
number.value = True
print(number)
number.value += 2**32
print(number.value)

print(ctypes.c_uint8(300), ctypes.c_int8(-129), ctypes.c_int16(70000))
print(ctypes.c_uint16(-1), ctypes.c_uint32(2**32 + 5), ctypes.c_int64(2**63))
print(ctypes.c_uint64(-1), ctypes.c_size_t(7), ctypes.c_ssize_t(-7))
print(ctypes.c_float(0.1), ctypes.c_double(1), ctypes.c_bool(5), ctypes.c_bool(""))

text = ctypes.c_char_p(b"hello")
print(text.value)
text.value = None
print(text.value, text)

address = ctypes.c_void_p()
print(address.value, address)
address.value = 16
print(address.value, address)
address.value = True
print(address.value)


def store(data: bytes | None) -> bytes | None:
    held = ctypes.c_char_p()
    held.value = data
    return held.value


print(store(b"kept"), store(None))

same = number
print(number == same, number == c_int(3), number is same, number != c_int(3))
try:
    print(ctypes.c_double(10**400))
except OverflowError as error:
    print("OverflowError:", error)
