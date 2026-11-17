"""Recursive calls: the Fibonacci numbers by their definition."""

import sys


def fibonacci(n: int) -> int:
    if n < 2:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)


def main() -> None:
    print(fibonacci(int(sys.argv[1])))


main()
