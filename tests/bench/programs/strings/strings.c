/* String work as programs do it: joining, splitting, case, searching and replacing, each step
   making a new string as Python's methods do. */
#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const char *WORDS[] = {"alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"};

static size_t occurrences(const char *text, const char *part) {
    size_t found = 0, length = strlen(part);
    for (const char *at = strstr(text, part); at; at = strstr(at + length, part)) found++;
    return found;
}

static char *replaced(const char *text, const char *old, const char *new) {
    size_t old_length = strlen(old), new_length = strlen(new);
    char *result = malloc(strlen(text) + occurrences(text, old) * new_length + 1), *out = result;
    for (const char *at; (at = strstr(text, old)); text = at + old_length) {
        memcpy(out, text, at - text);
        out += at - text;
        memcpy(out, new, new_length);
        out += new_length;
    }
    strcpy(out, text);
    return result;
}

int main(int argc, char **argv) {
    long n = argc > 1 ? atol(argv[1]) : 2000, total = 0;
    for (long i = 0; i < n; i++) {
        char line[128];
        size_t used = 0;
        for (int k = 0; k < 8; k++) {
            if (k) line[used++] = ' ';
            const char *word = WORDS[(i + k) % 7];
            size_t length = strlen(word);
            memcpy(line + used, word, length);
            used += length;
        }
        line[used] = '\0';
        char *upper = strdup(line);
        for (char *c = upper; *c; c++) *c = (char)toupper((unsigned char)*c);
        /* `split(" ")`: each piece is a string of its own. */
        size_t pieces = 0;
        char *rest = upper;
        for (char *space; (space = strchr(rest, ' ')); rest = space + 1) {
            free(strndup(rest, space - rest));
            pieces++;
        }
        free(strdup(rest));
        pieces++;
        const char *gamma = strstr(upper, "GAMMA");
        total += (long)pieces + (gamma ? gamma - upper : -1) + (long)occurrences(line, "a");
        char *shorter = replaced(line, "beta", "b");
        size_t start = 0, end = used;
        while (start < end && line[start] == 'a') start++;
        while (end > start && line[end - 1] == 'a') end--;
        char *stripped = strndup(line + start, end - start);
        total += (long)strlen(shorter) + (long)strlen(stripped);
        if (strncmp(line, "alpha", 5) == 0) total++;
        free(upper), free(shorter), free(stripped);
    }
    printf("%ld\n", total);
    return 0;
}
