// A small interpreter: arithmetic expressions written out, tokenized and evaluated.
#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

constexpr long MODULUS = 1000003;

// Python's `%`, whose result takes the divisor's sign.
static long modulo(long a, long m) { return ((a % m) + m) % m; }

static std::string expression(long seed) {
    std::vector<std::string> parts;
    for (long k = 0; k < 12; k++) {
        std::string value = std::to_string((seed * 31 + k * 17) % 97 + 1);
        parts.push_back(k % 4 == 0 ? "(" + value : k % 4 == 3 ? value + ")" : value);
    }
    const char *operators[] = {"+", "*", "-", "+"};
    std::string text = parts[0];
    for (long k = 1; k < 12; k++) text += std::string(" ") + operators[(seed + k) % 4] + " " + parts[k];
    return text;
}

static std::vector<std::string> tokens(const std::string &text) {
    std::vector<std::string> found;
    std::string number;
    for (char c : text) {
        if (std::isdigit((unsigned char)c)) {
            number += c;
            continue;
        }
        if (!number.empty()) {
            found.push_back(number);
            number.clear();
        }
        if (c != ' ') found.push_back(std::string(1, c));
    }
    if (!number.empty()) found.push_back(number);
    return found;
}

struct Parser {
    std::vector<std::string> items;
    size_t position = 0;

    std::string peek() const { return position < items.size() ? items[position] : ""; }
    std::string take() { return items[position++]; }

    long sum() {
        long value = product();
        while (peek() == "+" || peek() == "-") {
            if (take() == "+")
                value = modulo(value + product(), MODULUS);
            else
                value = modulo(value - product(), MODULUS);
        }
        return value;
    }

    long product() {
        long value = atom();
        while (peek() == "*") {
            take();
            value = modulo(value * atom(), MODULUS);
        }
        return value;
    }

    long atom() {
        std::string item = take();
        if (item == "(") {
            long value = sum();
            take();
            return value;
        }
        return std::stol(item);
    }
};

int main(int argc, char **argv) {
    long n = argc > 1 ? std::atol(argv[1]) : 2000, total = 0;
    for (long i = 0; i < n; i++) {
        Parser parser{tokens(expression(i))};
        total = modulo(total + parser.sum(), MODULUS);
    }
    std::printf("%ld\n", total);
}
