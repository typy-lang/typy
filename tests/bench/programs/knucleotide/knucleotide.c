/* The k-nucleotide of the Benchmarks Game, over a sequence of N drawn as fasta draws it. Each
   fragment is counted in an open-addressing table keyed by its FNV-1a hash. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define IM 139968
#define IA 3877
#define IC 29573

struct entry {
    const char *start;
    long count;
};

struct table {
    struct entry *slots;
    size_t capacity, size;
    int k;
};

static uint64_t hash(const char *text, int k) {
    uint64_t h = 1469598103934665603ULL;
    for (int i = 0; i < k; i++) h = (h ^ (unsigned char)text[i]) * 1099511628211ULL;
    return h;
}

static void grow(struct table *t) {
    struct table bigger = {calloc(t->capacity * 2, sizeof(struct entry)), t->capacity * 2, 0, t->k};
    for (size_t i = 0; i < t->capacity; i++) {
        if (!t->slots[i].start) continue;
        size_t j = hash(t->slots[i].start, t->k) & (bigger.capacity - 1);
        while (bigger.slots[j].start) j = (j + 1) & (bigger.capacity - 1);
        bigger.slots[j] = t->slots[i];
        bigger.size++;
    }
    free(t->slots);
    *t = bigger;
}

static struct entry *slot(struct table *t, const char *fragment) {
    if ((t->size + 1) * 2 > t->capacity) grow(t);
    size_t i = hash(fragment, t->k) & (t->capacity - 1);
    while (t->slots[i].start && memcmp(t->slots[i].start, fragment, t->k) != 0)
        i = (i + 1) & (t->capacity - 1);
    if (!t->slots[i].start) {
        t->slots[i].start = fragment;
        t->slots[i].count = 0;
        t->size++;
    }
    return &t->slots[i];
}

static struct table counts(const char *text, long length, int k) {
    struct table t = {calloc(16, sizeof(struct entry)), 16, 0, k};
    for (long i = 0; i + k <= length; i++) slot(&t, text + i)->count++;
    return t;
}

static int ranked_k;
static int ranked(const void *a, const void *b) {
    const struct entry *x = a, *y = b;
    if (x->count != y->count) return x->count > y->count ? -1 : 1;
    return memcmp(x->start, y->start, ranked_k);
}

int main(int argc, char **argv) {
    long n = argc > 1 ? atol(argv[1]) : 10000;
    static const double homo_sapiens[] = {0.3029549426680, 0.1979883004921, 0.1975473066391,
                                          0.3015094502008};
    double table[4], total = 0.0;
    for (int i = 0; i < 4; i++) {
        total += homo_sapiens[i];
        table[i] = total;
    }
    table[3] = 1.0;
    char *text = malloc(n + 1);
    long seed = 42;
    for (long i = 0; i < n; i++) {
        seed = (seed * IA + IC) % IM;
        double drawn = 1.0 * seed / IM;
        int k = 0;
        while (drawn >= table[k]) k++;
        text[i] = "ACGT"[k];
    }
    text[n] = '\0';
    for (int k = 1; k <= 2; k++) {
        struct table t = counts(text, n, k);
        struct entry *all = malloc(t.size * sizeof *all);
        size_t used = 0;
        for (size_t i = 0; i < t.capacity; i++)
            if (t.slots[i].start) all[used++] = t.slots[i];
        ranked_k = k;
        qsort(all, used, sizeof *all, ranked);
        for (size_t i = 0; i < used; i++)
            printf("%.*s %.3f\n", k, all[i].start, 100.0 * all[i].count / (n - k + 1));
        printf("\n");
        free(all);
        free(t.slots);
    }
    static const char *fragments[] = {"GGT", "GGTA", "GGTATT", "GGTATTTTAATT",
                                      "GGTATTTTAATTTATAGT"};
    for (int f = 0; f < 5; f++) {
        int k = (int)strlen(fragments[f]);
        struct table t = counts(text, n, k);
        long found = 0;
        size_t i = hash(fragments[f], k) & (t.capacity - 1);
        while (t.slots[i].start) {
            if (memcmp(t.slots[i].start, fragments[f], k) == 0) {
                found = t.slots[i].count;
                break;
            }
            i = (i + 1) & (t.capacity - 1);
        }
        printf("%ld\t%s\n", found, fragments[f]);
        free(t.slots);
    }
    free(text);
    return 0;
}
