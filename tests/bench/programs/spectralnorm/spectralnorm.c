/* The spectral norm of the Benchmarks Game: the largest eigenvalue of an infinite matrix. */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

static double element(long i, long j) { return 1.0 / ((i + j) * (i + j + 1) / 2 + i + 1); }

static void times(const double *u, double *v, long n) {
    for (long i = 0; i < n; i++) {
        double total = 0.0;
        for (long j = 0; j < n; j++) total += element(i, j) * u[j];
        v[i] = total;
    }
}

static void times_transposed(const double *u, double *v, long n) {
    for (long i = 0; i < n; i++) {
        double total = 0.0;
        for (long j = 0; j < n; j++) total += element(j, i) * u[j];
        v[i] = total;
    }
}

static void times_both(const double *u, double *v, double *scratch, long n) {
    times(u, scratch, n);
    times_transposed(scratch, v, n);
}

int main(int argc, char **argv) {
    long n = argc > 1 ? atol(argv[1]) : 100;
    double *u = malloc(n * sizeof *u), *v = malloc(n * sizeof *v);
    double *scratch = malloc(n * sizeof *scratch);
    for (long i = 0; i < n; i++) u[i] = 1.0, v[i] = 0.0, scratch[i] = 0.0;
    for (int k = 0; k < 10; k++) {
        times_both(u, v, scratch, n);
        times_both(v, u, scratch, n);
    }
    double vbv = 0.0, vv = 0.0;
    for (long i = 0; i < n; i++) {
        vbv += u[i] * v[i];
        vv += v[i] * v[i];
    }
    printf("%.9f\n", sqrt(vbv / vv));
    free(u), free(v), free(scratch);
    return 0;
}
