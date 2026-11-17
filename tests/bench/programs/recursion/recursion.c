/* Recursive calls: the Fibonacci numbers by their definition. */
#include <stdio.h>
#include <stdlib.h>

static long fibonacci(long n) { return n < 2 ? n : fibonacci(n - 1) + fibonacci(n - 2); }

int main(int argc, char **argv) {
    printf("%ld\n", fibonacci(argc > 1 ? atol(argv[1]) : 22));
    return 0;
}
