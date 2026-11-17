"""Dictionary work as programs do it: counting words, then ranking the most frequent."""

import sys


def count(n: int) -> dict[str, int]:
    found: dict[str, int] = {}
    seed = 1
    for _ in range(n):
        seed = (seed * 1103515245 + 12345) % 2147483648
        word = "w" + str(seed % 5000)
        found[word] = found.get(word, 0) + 1
    return found


def main() -> None:
    found = count(int(sys.argv[1]))
    ranked: list[tuple[int, str]] = []
    for word, times in found.items():
        ranked.append((-times, word))
    ranked.sort()
    print(len(found))
    for negative, word in ranked[:5]:
        print(f"{word} {-negative}")


main()
