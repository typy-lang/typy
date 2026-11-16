"""String work as programs do it: joining, splitting, case, searching and replacing."""

import sys

WORDS = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"]


def work(n: int) -> int:
    total = 0
    for i in range(n):
        parts: list[str] = []
        for k in range(8):
            parts.append(WORDS[(i + k) % len(WORDS)])
        line = " ".join(parts)
        upper = line.upper()
        pieces = upper.split(" ")
        total += len(pieces) + upper.find("GAMMA") + line.count("a")
        total += len(line.replace("beta", "b")) + len(line.strip("a"))
        if line.startswith("alpha"):
            total += 1
    return total


def main() -> None:
    print(work(int(sys.argv[1])))


main()
