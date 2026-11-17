/* The fannkuch-redux of the Benchmarks Game: flips of every permutation of N items. */
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    int n = argc > 1 ? atoi(argv[1]) : 7;
    int *first = malloc(n * sizeof *first), *perm = malloc(n * sizeof *perm);
    int *count = calloc(n, sizeof *count);
    long max_flips = 0, checksum = 0, permutations = 0;
    for (int i = 0; i < n; i++) first[i] = i;
    int r = n;
    for (;;) {
        while (r != 1) {
            count[r - 1] = r;
            r--;
        }
        for (int i = 0; i < n; i++) perm[i] = first[i];
        long flips = 0;
        int k = perm[0];
        while (k != 0) {
            for (int i = 0, j = k; i < j; i++, j--) {
                int held = perm[i];
                perm[i] = perm[j];
                perm[j] = held;
            }
            flips++;
            k = perm[0];
        }
        if (flips > max_flips) max_flips = flips;
        checksum += permutations % 2 == 0 ? flips : -flips;
        for (;;) {
            if (r == n) {
                printf("%ld\nPfannkuchen(%d) = %ld\n", checksum, n, max_flips);
                free(first), free(perm), free(count);
                return 0;
            }
            int head = first[0];
            for (int i = 0; i < r; i++) first[i] = first[i + 1];
            first[r] = head;
            count[r]--;
            if (count[r] > 0) break;
            r++;
        }
        permutations++;
    }
}
