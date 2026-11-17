"""The fannkuch-redux of the Benchmarks Game: flips of every permutation of N items."""

import sys


def fannkuch(n: int) -> tuple[int, int]:
    first = list(range(n))
    perm = [0] * n
    count = [0] * n
    max_flips = 0
    checksum = 0
    permutations = 0
    r = n
    while True:
        while r != 1:
            count[r - 1] = r
            r -= 1
        for i in range(n):
            perm[i] = first[i]
        flips = 0
        k = perm[0]
        while k != 0:
            i = 0
            j = k
            while i < j:
                held = perm[i]
                perm[i] = perm[j]
                perm[j] = held
                i += 1
                j -= 1
            flips += 1
            k = perm[0]
        if flips > max_flips:
            max_flips = flips
        if permutations % 2 == 0:
            checksum += flips
        else:
            checksum -= flips
        while True:
            if r == n:
                return (checksum, max_flips)
            head = first[0]
            for i in range(r):
                first[i] = first[i + 1]
            first[r] = head
            count[r] -= 1
            if count[r] > 0:
                break
            r += 1
        permutations += 1


def main() -> None:
    n = int(sys.argv[1])
    checksum, max_flips = fannkuch(n)
    print(checksum)
    print(f"Pfannkuchen({n}) = {max_flips}")


main()
