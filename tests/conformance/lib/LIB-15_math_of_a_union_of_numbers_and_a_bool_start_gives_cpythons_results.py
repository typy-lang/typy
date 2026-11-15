# typy-test: LIB-15
# typy-expect: accepted
import math
mixed: list[int | float | bool] = [1, 2.5, True]
zeros: list[int | float] = [0, 0, 0]
print(math.fsum(mixed), math.prod(mixed), math.sumprod(mixed, mixed), math.dist(mixed, zeros))
nothing: list[int] = []
print(math.prod([2, 3], start=True), math.prod(nothing, start=True), math.prod([True, True], start=True))
pair: list[int | float] = [2.5, 2]
print(math.prod(pair, start=2), math.prod(pair, start=False))
ints: list[int | float] = [3, 2]
halves: list[int | float] = [1.5, 2]
print(math.prod(ints), math.sumprod(ints, ints), math.sumprod(ints, halves))
big: list[int | float] = [3, 2**60 + 1, 1.0]
print(math.prod(big))
p: list[int | float] = [2**62, 2**62, 1.5]
q: list[int | float] = [4, 4, 1.0]
print(math.sumprod(p, q))
r: list[int | float] = [1.0, 2**53 + 1]
s: list[int | float] = [1.0, 1]
print(math.sumprod(r, s))
flags: list[bool | int] = [True, 2]
print(math.sumprod(flags, [3, 4]), math.prod(flags))
empty: list[int | float] = []
print(math.prod(empty), math.sumprod(empty, empty), math.prod(empty, start=True))
huge: list[int | float] = [10**400, 1.0]
ones: list[int | float] = [1, 1.0]
try:
    print(math.prod(huge))
except OverflowError as error:
    print("OverflowError:", error)
try:
    print(math.sumprod(huge, ones))
except OverflowError as error:
    print("OverflowError:", error)
one: list[int | float] = [1.0]
try:
    print(math.sumprod(ints, one))
except ValueError as error:
    print("ValueError:", error)
