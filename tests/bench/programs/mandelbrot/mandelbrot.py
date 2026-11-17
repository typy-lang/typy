"""The Mandelbrot set of the Benchmarks Game, as the count of points of an N by N grid in it."""

import sys


def inside(n: int) -> int:
    found = 0
    for y in range(n):
        ci = 2.0 * y / n - 1.0
        for x in range(n):
            cr = 2.0 * x / n - 1.5
            zr = 0.0
            zi = 0.0
            escaped = False
            for _ in range(50):
                next_r = zr * zr - zi * zi + cr
                zi = 2.0 * zr * zi + ci
                zr = next_r
                if zr * zr + zi * zi > 4.0:
                    escaped = True
                    break
            if not escaped:
                found += 1
    return found


def main() -> None:
    n = int(sys.argv[1])
    print(inside(n))


main()
