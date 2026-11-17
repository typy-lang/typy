"""The k-nucleotide of the Benchmarks Game, over a sequence of N drawn as fasta draws it."""

import sys

IM = 139968
IA = 3877
IC = 29573
HOMO_SAPIENS = [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008]
FRAGMENTS = ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]


def sequence(n: int) -> str:
    table: list[float] = []
    total = 0.0
    for probability in HOMO_SAPIENS:
        total += probability
        table.append(total)
    table[3] = 1.0
    seed = 42
    characters: list[str] = []
    for _ in range(n):
        seed = (seed * IA + IC) % IM
        drawn = 1.0 * seed / IM
        k = 0
        while drawn >= table[k]:
            k += 1
        characters.append("ACGT"[k])
    return "".join(characters)


def counts(text: str, k: int) -> dict[str, int]:
    found: dict[str, int] = {}
    for i in range(len(text) - k + 1):
        fragment = text[i : i + k]
        found[fragment] = found.get(fragment, 0) + 1
    return found


def main() -> None:
    text = sequence(int(sys.argv[1]))
    for k in [1, 2]:
        found = counts(text, k)
        total = len(text) - k + 1
        ranked: list[tuple[int, str]] = []
        for fragment, count in found.items():
            ranked.append((-count, fragment))
        ranked.sort()
        for negative, fragment in ranked:
            print(f"{fragment} {100.0 * -negative / total:.3f}")
        print("")
    for fragment in FRAGMENTS:
        print(f"{counts(text, len(fragment)).get(fragment, 0)}\t{fragment}")


main()
