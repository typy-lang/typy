// The k-nucleotide of the Benchmarks Game, over a sequence of N drawn as fasta draws it.
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <string_view>
#include <unordered_map>
#include <utility>
#include <vector>

constexpr long IM = 139968, IA = 3877, IC = 29573;

static std::string sequence(long n) {
    const double homo_sapiens[] = {0.3029549426680, 0.1979883004921, 0.1975473066391,
                                   0.3015094502008};
    double table[4], total = 0.0;
    for (int i = 0; i < 4; i++) table[i] = total += homo_sapiens[i];
    table[3] = 1.0;
    std::string text;
    long seed = 42;
    for (long i = 0; i < n; i++) {
        seed = (seed * IA + IC) % IM;
        double drawn = 1.0 * seed / IM;
        int k = 0;
        while (drawn >= table[k]) k++;
        text += "ACGT"[k];
    }
    return text;
}

static std::unordered_map<std::string_view, long> counts(const std::string &text, size_t k) {
    std::unordered_map<std::string_view, long> found;
    for (size_t i = 0; i + k <= text.size(); i++) found[std::string_view(text).substr(i, k)]++;
    return found;
}

int main(int argc, char **argv) {
    std::string text = sequence(argc > 1 ? std::atol(argv[1]) : 10000);
    for (size_t k = 1; k <= 2; k++) {
        std::vector<std::pair<long, std::string_view>> ranked;
        for (auto [fragment, count] : counts(text, k)) ranked.emplace_back(-count, fragment);
        std::sort(ranked.begin(), ranked.end());
        double total = text.size() - k + 1;
        for (auto [negative, fragment] : ranked)
            std::printf("%.*s %.3f\n", (int)fragment.size(), fragment.data(),
                        100.0 * -negative / total);
        std::printf("\n");
    }
    for (const char *fragment :
         {"GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"}) {
        auto found = counts(text, std::string_view(fragment).size());
        auto at = found.find(fragment);
        std::printf("%ld\t%s\n", at == found.end() ? 0 : at->second, fragment);
    }
}
