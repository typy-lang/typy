// The fannkuch-redux of the Benchmarks Game: flips of every permutation of N items.
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <numeric>
#include <vector>

int main(int argc, char **argv) {
    int n = argc > 1 ? std::atoi(argv[1]) : 7;
    std::vector<int> first(n), perm(n), count(n, 0);
    std::iota(first.begin(), first.end(), 0);
    long max_flips = 0, checksum = 0, permutations = 0;
    int r = n;
    for (;;) {
        for (; r != 1; r--) count[r - 1] = r;
        perm = first;
        long flips = 0;
        for (int k = perm[0]; k != 0; k = perm[0]) {
            std::reverse(perm.begin(), perm.begin() + k + 1);
            flips++;
        }
        max_flips = std::max(max_flips, flips);
        checksum += permutations % 2 == 0 ? flips : -flips;
        for (;;) {
            if (r == n) {
                std::printf("%ld\nPfannkuchen(%d) = %ld\n", checksum, n, max_flips);
                return 0;
            }
            std::rotate(first.begin(), first.begin() + 1, first.begin() + r + 1);
            if (--count[r] > 0) break;
            r++;
        }
        permutations++;
    }
}
