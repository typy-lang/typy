# typy-test: FFI-11
# typy-expect: rejected E0201 at 8:5
import ctypes


class Point(ctypes.Structure):
    _fields_ = [("x", ctypes.c_int)]
    def doubled(self) -> int:
        return self.x * 2
