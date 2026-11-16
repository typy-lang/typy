"""The fasta of the Benchmarks Game: DNA sequences, repeated and drawn from a generator."""

import sys

IM = 139968
IA = 3877
IC = 29573
WIDTH = 60
ALU = (
    "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAG"
    "ACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCC"
    "AGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
    "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
)
IUB = [0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02]
HOMO_SAPIENS = [0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008]


class Random:
    def __init__(self) -> None:
        self.seed = 42

    def draw(self, maximum: float) -> float:
        self.seed = (self.seed * IA + IC) % IM
        return maximum * self.seed / IM


def cumulative(probabilities: list[float]) -> list[float]:
    table: list[float] = []
    total = 0.0
    for probability in probabilities:
        total += probability
        table.append(total)
    table[len(table) - 1] = 1.0
    return table


def repeat_fasta(header: str, source: str, n: int) -> None:
    print(header)
    extended = source + source[:WIDTH]
    length = len(source)
    position = 0
    while n > 0:
        line = min(WIDTH, n)
        print(extended[position : position + line])
        position = (position + line) % length
        n -= line


def random_fasta(
    header: str, symbols: str, probabilities: list[float], n: int, random: Random
) -> None:
    print(header)
    table = cumulative(probabilities)
    while n > 0:
        line = min(WIDTH, n)
        characters: list[str] = []
        for _ in range(line):
            drawn = random.draw(1.0)
            k = 0
            while drawn >= table[k]:
                k += 1
            characters.append(symbols[k])
        print("".join(characters))
        n -= line


def main() -> None:
    n = int(sys.argv[1])
    repeat_fasta(">ONE Homo sapiens alu", ALU, 2 * n)
    random = Random()
    random_fasta(">TWO IUB ambiguity codes", "acgtBDHKMNRSVWY", IUB, 3 * n, random)
    random_fasta(">THREE Homo sapiens frequency", "acgt", HOMO_SAPIENS, 5 * n, random)


main()
