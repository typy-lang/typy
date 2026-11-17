# typy-test: FFI-03
# typy-expect: rejected E0404 at 9:13
import ctypes


class Point(ctypes.Structure):
    _fields_ = [("x", ctypes.c_int), ("y", ctypes.c_int)]

print(Point(z=1).x)
