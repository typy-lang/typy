// The Mandelbrot set of the Benchmarks Game, as the count of points of an N by N grid in it.
#include <cstdio>
#include <cstdlib>

int main(int argc, char **argv) {
    long n = argc > 1 ? std::atol(argv[1]) : 200, found = 0;
    for (long y = 0; y < n; y++) {
        double ci = 2.0 * y / n - 1.0;
        for (long x = 0; x < n; x++) {
            double cr = 2.0 * x / n - 1.5, zr = 0.0, zi = 0.0;
            bool escaped = false;
            for (int i = 0; i < 50; i++) {
                double next_r = zr * zr - zi * zi + cr;
                zi = 2.0 * zr * zi + ci;
                zr = next_r;
                if (zr * zr + zi * zi > 4.0) {
                    escaped = true;
                    break;
                }
            }
            if (!escaped) found++;
        }
    }
    std::printf("%ld\n", found);
}
