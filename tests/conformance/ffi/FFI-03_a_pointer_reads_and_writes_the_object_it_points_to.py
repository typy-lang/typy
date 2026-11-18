# typy-test: FFI-03
# typy-expect: accepted
import ctypes

libc = ctypes.CDLL(None)
libc.calloc.argtypes = [ctypes.c_size_t, ctypes.c_size_t]
libc.calloc.restype = ctypes.POINTER(ctypes.c_int)
libc.free.argtypes = [ctypes.POINTER(ctypes.c_int)]
libc.free.restype = None

value = ctypes.c_long(9)
pointer = ctypes.pointer(value)
print(pointer.contents, pointer[0], bool(pointer))

pointer[0] = 10
print(value.value)
pointer.contents.value = 11
print(value)
pointer[0] += 1
print(value.value, pointer.contents.value)

# A pointer keeps its target alive after the name is gone.
other = ctypes.POINTER(ctypes.c_double)(ctypes.c_double(2.5))
print(other[0], other.contents)

empty = ctypes.POINTER(ctypes.c_int)()
print(bool(empty), ctypes.sizeof(empty))
try:
    print(empty.contents)
except ValueError as error:
    print("ValueError:", error)
try:
    print(empty[0])
except ValueError as error:
    print("ValueError:", error)

# A pointer that C gives reads memory C owns.
owned = libc.calloc(2, ctypes.sizeof(ctypes.c_int))
print(bool(owned), owned[0], owned[1])
owned.contents.value = 34
owned[1] = -5
print(owned[0], owned[1], owned.contents.value)
libc.free(owned)
