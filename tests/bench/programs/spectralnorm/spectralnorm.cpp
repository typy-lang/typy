// The spectral norm of the Benchmarks Game: the largest eigenvalue of an infinite matrix.
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <vector>

static double element(long i, long j) { return 1.0 / ((i + j) * (i + j + 1) / 2 + i + 1); }

static void times(const std::vector<double> &u, std::vector<double> &v) {
    for (size_t i = 0; i < v.size(); i++) {
        double total = 0.0;
        for (size_t j = 0; j < u.size(); j++) total += element(i, j) * u[j];
        v[i] = total;
    }
}

static void times_transposed(const std::vector<double> &u, std::vector<double> &v) {
    for (size_t i = 0; i < v.size(); i++) {
        double total = 0.0;
        for (size_t j = 0; j < u.size(); j++) total += element(j, i) * u[j];
        v[i] = total;
    }
}

int main(int argc, char **argv) {
    long n = argc > 1 ? std::atol(argv[1]) : 100;
    std::vector<double> u(n, 1.0), v(n, 0.0), scratch(n, 0.0);
    for (int k = 0; k < 10; k++) {
        times(u, scratch);
        times_transposed(scratch, v);
        times(v, scratch);
        times_transposed(scratch, u);
    }
    double vbv = 0.0, vv = 0.0;
    for (long i = 0; i < n; i++) {
        vbv += u[i] * v[i];
        vv += v[i] * v[i];
    }
    std::printf("%.9f\n", std::sqrt(vbv / vv));
}
