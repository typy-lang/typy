// Dictionary work as programs do it: counting words, then ranking the most frequent.
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

int main(int argc, char **argv) {
    long n = argc > 1 ? std::atol(argv[1]) : 20000;
    std::unordered_map<std::string, long> found;
    long long seed = 1;
    for (long i = 0; i < n; i++) {
        seed = (seed * 1103515245 + 12345) % 2147483648LL;
        found["w" + std::to_string(seed % 5000)]++;
    }
    std::vector<std::pair<long, std::string>> ranked;
    for (const auto &[word, count] : found) ranked.emplace_back(-count, word);
    std::sort(ranked.begin(), ranked.end());
    std::printf("%zu\n", found.size());
    for (size_t i = 0; i < 5 && i < ranked.size(); i++)
        std::printf("%s %ld\n", ranked[i].second.c_str(), -ranked[i].first);
}
