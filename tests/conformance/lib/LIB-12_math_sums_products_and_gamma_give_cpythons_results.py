# typy-test: LIB-12
# typy-expect: accepted
# typy-output: architecture-dependent
# CPython's gamma and lgamma round differently on AArch64, where its compiler fuses
# their multiply-adds.
import math

print(math.gamma(0.5), math.gamma(5), math.gamma(-1.5), math.gamma(171.5), math.gamma(1e-300))
print(math.lgamma(0.5), math.lgamma(100.0), math.lgamma(-2.5), math.lgamma(1e-30))
print(math.fsum([0.1] * 10), math.fsum([1e100, 1.0, -1e100, 1e-100]), math.fsum([1, 2, 3]))
print(math.fsum([True, False]), math.fsum(range(5)), math.fsum((1e-16, 1.0, 1e16)))
print(math.dist([0, 0], [3, 4]), math.dist((1.5, 2.5, 3.5), (0.0, 0.0, 0.0)))
print(math.prod([1, 2, 3]), math.prod([2] * 70), math.prod([1.5, 2.0]), math.prod([2.0], start=3))
empty: list[float] = []
print(math.prod(empty), math.prod(empty, start=2.5), math.prod(range(1, 6), start=2))
print(math.sumprod([1, 2], [3, 4]), math.sumprod([0.1, 0.2], [3.0, 4.0]), math.sumprod(empty, empty))
print(math.sumprod([1.5, 2.5], [2, 3]), math.sumprod([10**20, 3], [10**20, 4]))
print(math.sumprod([1e308, 1e308], [10.0, -10.0]), math.sumprod([True, False], [2, 5]))


def guarded(values: list[float]) -> None:
    try:
        print(math.fsum(values))
    except OverflowError as error:
        print("OverflowError", error)
    except ValueError as error:
        print("ValueError", type(error).__name__)


guarded([1e308, 1e308])
guarded([float("inf"), float("-inf")])
try:
    print(math.dist([1.0], [1.0, 2.0]))
except ValueError as error:
    print("ValueError", error)
try:
    print(math.sumprod([1.0, 2.0], [3.0]))
except ValueError as error:
    print("ValueError", error)
try:
    print(math.sumprod([1.5], [10**400]))
except OverflowError as error:
    print("OverflowError", error)
try:
    print(math.gamma(-2.0))
except ValueError:
    print("ValueError")
