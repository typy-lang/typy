# typy-test: FFI-10
# typy-expect: rejected E0201 at 5:11
import ctypes

pointer = ctypes.pointer(ctypes.pointer(ctypes.c_int(1)))
