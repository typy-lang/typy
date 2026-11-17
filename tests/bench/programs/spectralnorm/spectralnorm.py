"""The spectral norm of the Benchmarks Game: the largest eigenvalue of an infinite matrix."""

import math
import sys


def element(i: int, j: int) -> float:
    return 1.0 / ((i + j) * (i + j + 1) // 2 + i + 1)


def times(u: list[float], v: list[float]) -> None:
    n = len(u)
    for i in range(n):
        total = 0.0
        for j in range(n):
            total += element(i, j) * u[j]
        v[i] = total


def times_transposed(u: list[float], v: list[float]) -> None:
    n = len(u)
    for i in range(n):
        total = 0.0
        for j in range(n):
            total += element(j, i) * u[j]
        v[i] = total


def times_both(u: list[float], v: list[float], scratch: list[float]) -> None:
    times(u, scratch)
    times_transposed(scratch, v)


def main() -> None:
    n = int(sys.argv[1])
    u = [1.0] * n
    v = [0.0] * n
    scratch = [0.0] * n
    for _ in range(10):
        times_both(u, v, scratch)
        times_both(v, u, scratch)
    vbv = 0.0
    vv = 0.0
    for i in range(n):
        vbv += u[i] * v[i]
        vv += v[i] * v[i]
    print(f"{math.sqrt(vbv / vv):.9f}")


main()
