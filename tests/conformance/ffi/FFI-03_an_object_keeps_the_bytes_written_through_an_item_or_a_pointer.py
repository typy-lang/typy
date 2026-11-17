# typy-test: FFI-03
# typy-expect: accepted
import ctypes

texts = (ctypes.c_char_p * 2)()
texts[0] = bytes([104, 105])
texts[1] = b"x" * 3
print(texts[0], texts[1], list(texts))
single = ctypes.c_char_p()
single.value = bytes([65])
print(single.value)
pointer = ctypes.pointer(single)
pointer[0] = bytes([66, 67])
print(single.value, pointer.contents.value)
pointer.contents.value = bytes([68])
print(single.value)
texts[0] = None
print(list(texts))
