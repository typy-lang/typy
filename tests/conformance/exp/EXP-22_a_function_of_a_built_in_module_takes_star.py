# typy-test: EXP-22
# typy-expect: accepted
import math
import sys
from math import floor, gcd


def rounded(values: list[float]) -> str:
    try:
        return str(math.floor(*values))
    except TypeError as error:
        return str(error)


def near(values: list[float]) -> str:
    try:
        return str(math.isclose(*values))
    except TypeError as error:
        return str(error)


def product(groups: list[list[int]]) -> str:
    try:
        return str(math.prod(*groups))
    except TypeError as error:
        return str(error)


print(rounded([2.5]), rounded([]), rounded([1.5, 2.5]), floor(*[3.7]))
print(near([]), near([1.0]), near([1.0, 1.0]), near([1.0, 1.0, 2.0]), near([1.0] * 5))
print(product([[2, 3]]), product([[1], [2]]), product([[1], [2], [3]]))
numbers = [12, 18, 30]
none: list[int] = []
nothing: list[float] = []
print(math.gcd(*numbers), gcd(*none), math.lcm(*numbers), math.lcm(*[4]))
print(math.hypot(*[3.0, 4.0]), math.hypot(*[3, 4]), math.hypot(*nothing))
print(math.pow(*[2.0, 3.0]), math.log(*[8.0, 2.0]), sys.getrecursionlimit(*none) > 0)
words = ["b", "a"]
try:
    words.sort(*[1, 2, 3])
except TypeError as error:
    print(error)
try:
    words.sort(*[1])
except TypeError as error:
    print(error)
codes: list[int] = []
try:
    sys.exit(*codes)
except SystemExit as stop:
    print("exit", stop.code)
