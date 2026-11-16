/* A small interpreter: arithmetic expressions written out, tokenized and evaluated. */
#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MODULUS 1000003

/* Python's `%`, whose result takes the divisor's sign. */
static long modulo(long a, long m) { return ((a % m) + m) % m; }

static char *expression(long seed) {
    static const char operators[] = {'+', '*', '-', '+'};
    char parts[12][16];
    for (int k = 0; k < 12; k++) {
        long value = (seed * 31 + k * 17) % 97 + 1;
        if (k % 4 == 0)
            snprintf(parts[k], sizeof parts[k], "(%ld", value);
        else if (k % 4 == 3)
            snprintf(parts[k], sizeof parts[k], "%ld)", value);
        else
            snprintf(parts[k], sizeof parts[k], "%ld", value);
    }
    char *text = malloc(256);
    size_t used = (size_t)snprintf(text, 256, "%s", parts[0]);
    for (int k = 1; k < 12; k++)
        used += (size_t)snprintf(text + used, 256 - used, " %c %s", operators[(seed + k) % 4],
                                 parts[k]);
    return text;
}

struct tokens {
    char **items;
    int count, position;
};

static struct tokens tokenize(const char *text) {
    struct tokens found = {malloc(64 * sizeof(char *)), 0, 0};
    char number[16];
    int digits = 0;
    for (const char *c = text;; c++) {
        if (*c && isdigit((unsigned char)*c)) {
            number[digits++] = *c;
            continue;
        }
        if (digits) {
            number[digits] = '\0';
            found.items[found.count++] = strdup(number);
            digits = 0;
        }
        if (!*c) break;
        if (*c != ' ') {
            char *one = malloc(2);
            one[0] = *c, one[1] = '\0';
            found.items[found.count++] = one;
        }
    }
    return found;
}

static const char *peek(const struct tokens *t) {
    return t->position < t->count ? t->items[t->position] : "";
}
static const char *take(struct tokens *t) { return t->items[t->position++]; }

static long sum(struct tokens *t);

static long atom(struct tokens *t) {
    const char *item = take(t);
    if (item[0] == '(') {
        long value = sum(t);
        take(t);
        return value;
    }
    return atol(item);
}

static long product(struct tokens *t) {
    long value = atom(t);
    while (strcmp(peek(t), "*") == 0) {
        take(t);
        value = modulo(value * atom(t), MODULUS);
    }
    return value;
}

static long sum(struct tokens *t) {
    long value = product(t);
    while (strcmp(peek(t), "+") == 0 || strcmp(peek(t), "-") == 0) {
        if (take(t)[0] == '+')
            value = modulo(value + product(t), MODULUS);
        else
            value = modulo(value - product(t), MODULUS);
    }
    return value;
}

int main(int argc, char **argv) {
    long n = argc > 1 ? atol(argv[1]) : 2000, total = 0;
    for (long i = 0; i < n; i++) {
        char *text = expression(i);
        struct tokens t = tokenize(text);
        total = modulo(total + sum(&t), MODULUS);
        for (int k = 0; k < t.count; k++) free(t.items[k]);
        free(t.items);
        free(text);
    }
    printf("%ld\n", total);
    return 0;
}
