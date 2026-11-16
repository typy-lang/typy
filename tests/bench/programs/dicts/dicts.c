/* Dictionary work as programs do it: counting words, then ranking the most frequent. Words are
   kept in an open-addressing table keyed by their FNV-1a hash. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct entry {
    char *word;
    long count;
};

struct table {
    struct entry *slots;
    size_t capacity, size;
};

static uint64_t hash(const char *text) {
    uint64_t h = 1469598103934665603ULL;
    for (; *text; text++) h = (h ^ (unsigned char)*text) * 1099511628211ULL;
    return h;
}

static void grow(struct table *t);

static struct entry *slot(struct table *t, const char *word) {
    if ((t->size + 1) * 2 > t->capacity) grow(t);
    size_t i = hash(word) & (t->capacity - 1);
    while (t->slots[i].word && strcmp(t->slots[i].word, word) != 0) i = (i + 1) & (t->capacity - 1);
    if (!t->slots[i].word) {
        t->slots[i].word = strdup(word);
        t->slots[i].count = 0;
        t->size++;
    }
    return &t->slots[i];
}

static void grow(struct table *t) {
    struct table bigger = {calloc(t->capacity * 2, sizeof(struct entry)), t->capacity * 2, 0};
    for (size_t i = 0; i < t->capacity; i++) {
        if (!t->slots[i].word) continue;
        size_t j = hash(t->slots[i].word) & (bigger.capacity - 1);
        while (bigger.slots[j].word) j = (j + 1) & (bigger.capacity - 1);
        bigger.slots[j] = t->slots[i];
        bigger.size++;
    }
    free(t->slots);
    *t = bigger;
}

static int ranked(const void *a, const void *b) {
    const struct entry *x = a, *y = b;
    if (x->count != y->count) return x->count > y->count ? -1 : 1;
    return strcmp(x->word, y->word);
}

int main(int argc, char **argv) {
    long n = argc > 1 ? atol(argv[1]) : 20000;
    struct table t = {calloc(16, sizeof(struct entry)), 16, 0};
    long long seed = 1;
    char word[32];
    for (long i = 0; i < n; i++) {
        seed = (seed * 1103515245 + 12345) % 2147483648LL;
        snprintf(word, sizeof word, "w%lld", seed % 5000);
        slot(&t, word)->count++;
    }
    struct entry *all = malloc(t.size * sizeof *all);
    size_t used = 0;
    for (size_t i = 0; i < t.capacity; i++)
        if (t.slots[i].word) all[used++] = t.slots[i];
    qsort(all, used, sizeof *all, ranked);
    printf("%zu\n", used);
    for (size_t i = 0; i < 5 && i < used; i++) printf("%s %ld\n", all[i].word, all[i].count);
    for (size_t i = 0; i < used; i++) free(all[i].word);
    free(all);
    free(t.slots);
    return 0;
}
