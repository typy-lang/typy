/* The fasta of the Benchmarks Game: DNA sequences, repeated and drawn from a generator. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define IM 139968
#define IA 3877
#define IC 29573
#define WIDTH 60

static const char *ALU =
    "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAG"
    "ACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCC"
    "AGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
    "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

static long seed = 42;

static double draw(double maximum) {
    seed = (seed * IA + IC) % IM;
    return maximum * seed / IM;
}

static void repeat_fasta(const char *header, const char *source, long n) {
    puts(header);
    size_t length = strlen(source);
    char *extended = malloc(length + WIDTH + 1);
    memcpy(extended, source, length);
    memcpy(extended + length, source, WIDTH);
    extended[length + WIDTH] = '\0';
    char line[WIDTH + 1];
    size_t position = 0;
    while (n > 0) {
        long size = n < WIDTH ? n : WIDTH;
        memcpy(line, extended + position, size);
        line[size] = '\0';
        puts(line);
        position = (position + size) % length;
        n -= size;
    }
    free(extended);
}

static void random_fasta(const char *header, const char *symbols, const double *probabilities,
                         int count, long n) {
    puts(header);
    double table[16], total = 0.0;
    for (int i = 0; i < count; i++) {
        total += probabilities[i];
        table[i] = total;
    }
    table[count - 1] = 1.0;
    char line[WIDTH + 1];
    while (n > 0) {
        long size = n < WIDTH ? n : WIDTH;
        for (long i = 0; i < size; i++) {
            double drawn = draw(1.0);
            int k = 0;
            while (drawn >= table[k]) k++;
            line[i] = symbols[k];
        }
        line[size] = '\0';
        puts(line);
        n -= size;
    }
}

int main(int argc, char **argv) {
    long n = argc > 1 ? atol(argv[1]) : 1000;
    static const double iub[] = {0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02,
                                 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02};
    static const double homo_sapiens[] = {0.3029549426680, 0.1979883004921, 0.1975473066391,
                                          0.3015094502008};
    repeat_fasta(">ONE Homo sapiens alu", ALU, 2 * n);
    random_fasta(">TWO IUB ambiguity codes", "acgtBDHKMNRSVWY", iub, 15, 3 * n);
    random_fasta(">THREE Homo sapiens frequency", "acgt", homo_sapiens, 4, 5 * n);
    return 0;
}
