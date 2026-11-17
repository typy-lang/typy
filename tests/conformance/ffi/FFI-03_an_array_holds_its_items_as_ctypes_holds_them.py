# typy-test: FFI-03
# typy-expect: accepted
import ctypes

Numbers = ctypes.c_int * 4
numbers = Numbers(1, 2, 3)
print(len(numbers), numbers[0], numbers[-1], list(numbers))
print(ctypes.sizeof(numbers), ctypes.sizeof(Numbers), bool(numbers))

numbers[3] = 2**32 + 40
total = 0
for each in numbers:
    total += each
print(total, sum(numbers), sorted(numbers, reverse=True))
print(3 in numbers, 7 in numbers, list(reversed(numbers)))
print([(index, each) for index, each in enumerate(numbers)])

try:
    print(numbers[4])
except IndexError as error:
    print("IndexError:", error)
try:
    numbers[-5] = 1
except IndexError as error:
    print("IndexError:", error)
try:
    Numbers(1, 2, 3, 4, 5)
except IndexError as error:
    print("IndexError:", error)

print(list((ctypes.c_double * 2)(1, 2.5)), list((ctypes.c_bool * 3)(1, 0, "x")))
print(list((ctypes.c_char_p * 2)(b"a")), list((ctypes.c_uint8 * 2)(256, -1)))
print(len((ctypes.c_int * 0)()), bool((ctypes.c_int * 0)()))
