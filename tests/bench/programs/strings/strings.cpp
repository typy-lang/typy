// String work as programs do it: joining, splitting, case, searching and replacing.
#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

static const char *WORDS[] = {"alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"};

static std::string replaced(std::string text, const std::string &old, const std::string &now) {
    for (size_t at = text.find(old); at != std::string::npos; at = text.find(old, at + now.size()))
        text.replace(at, old.size(), now);
    return text;
}

int main(int argc, char **argv) {
    long n = argc > 1 ? std::atol(argv[1]) : 2000, total = 0;
    for (long i = 0; i < n; i++) {
        std::string line;
        for (int k = 0; k < 8; k++) {
            if (k) line += ' ';
            line += WORDS[(i + k) % 7];
        }
        std::string upper = line;
        for (char &c : upper) c = (char)std::toupper((unsigned char)c);
        std::vector<std::string> pieces;
        size_t start = 0;
        for (size_t space; (space = upper.find(' ', start)) != std::string::npos; start = space + 1)
            pieces.push_back(upper.substr(start, space - start));
        pieces.push_back(upper.substr(start));
        size_t gamma = upper.find("GAMMA");
        long a = 0;
        for (char c : line) a += c == 'a';
        total += (long)pieces.size() + (gamma == std::string::npos ? -1 : (long)gamma) + a;
        size_t first = line.find_first_not_of('a'), last = line.find_last_not_of('a');
        std::string stripped = first == std::string::npos ? "" : line.substr(first, last - first + 1);
        total += (long)replaced(line, "beta", "b").size() + (long)stripped.size();
        if (line.rfind("alpha", 0) == 0) total++;
    }
    std::printf("%ld\n", total);
}
