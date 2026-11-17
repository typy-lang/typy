// The fasta of the Benchmarks Game: DNA sequences, repeated and drawn from a generator.
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

constexpr long IM = 139968, IA = 3877, IC = 29573;
constexpr long WIDTH = 60;
static const std::string ALU =
    "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAG"
    "ACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCC"
    "AGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
    "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

struct Random {
    long seed = 42;
    double draw(double maximum) {
        seed = (seed * IA + IC) % IM;
        return maximum * seed / IM;
    }
};

static void repeat_fasta(const char *header, const std::string &source, long n) {
    std::puts(header);
    std::string extended = source + source.substr(0, WIDTH);
    size_t position = 0;
    while (n > 0) {
        long line = std::min(WIDTH, n);
        std::puts(extended.substr(position, line).c_str());
        position = (position + line) % source.size();
        n -= line;
    }
}

static void random_fasta(const char *header, const std::string &symbols,
                         const std::vector<double> &probabilities, long n, Random &random) {
    std::puts(header);
    std::vector<double> table;
    double total = 0.0;
    for (double p : probabilities) table.push_back(total += p);
    table.back() = 1.0;
    std::string line;
    while (n > 0) {
        long size = std::min(WIDTH, n);
        line.clear();
        for (long i = 0; i < size; i++) {
            double drawn = random.draw(1.0);
            size_t k = 0;
            while (drawn >= table[k]) k++;
            line += symbols[k];
        }
        std::puts(line.c_str());
        n -= size;
    }
}

int main(int argc, char **argv) {
    long n = argc > 1 ? std::atol(argv[1]) : 1000;
    repeat_fasta(">ONE Homo sapiens alu", ALU, 2 * n);
    Random random;
    random_fasta(">TWO IUB ambiguity codes", "acgtBDHKMNRSVWY",
                 {0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02,
                  0.02, 0.02},
                 3 * n, random);
    random_fasta(">THREE Homo sapiens frequency", "acgt",
                 {0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008}, 5 * n,
                 random);
}
