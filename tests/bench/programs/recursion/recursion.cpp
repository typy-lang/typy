// Recursive calls: the Fibonacci numbers by their definition.
#include <cstdio>
#include <cstdlib>

static long fibonacci(long n) { return n < 2 ? n : fibonacci(n - 1) + fibonacci(n - 2); }

int main(int argc, char **argv) {
    std::printf("%ld\n", fibonacci(argc > 1 ? std::atol(argv[1]) : 22));
}
