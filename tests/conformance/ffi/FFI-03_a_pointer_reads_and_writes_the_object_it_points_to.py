# typy-test: FFI-03
# typy-expect: accepted
import ctypes

libc = ctypes.CDLL("libc.so.6")
libc.__errno_location.argtypes = []
libc.__errno_location.restype = ctypes.POINTER(ctypes.c_int)

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
errno = libc.__errno_location()
errno.contents.value = 0
print(bool(errno), errno[0])
errno[0] = 34
print(libc.__errno_location()[0])
errno[0] = 0
