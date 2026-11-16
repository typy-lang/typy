/* The binary trees of the Benchmarks Game: many short-lived trees, allocated and freed. */
#include <stdio.h>
#include <stdlib.h>

struct node {
    struct node *left, *right;
};

static struct node *make(int depth) {
    struct node *node = malloc(sizeof *node);
    if (depth == 0) {
        node->left = node->right = NULL;
    } else {
        node->left = make(depth - 1);
        node->right = make(depth - 1);
    }
    return node;
}

static long check(const struct node *node) {
    if (node->left == NULL || node->right == NULL) return 1;
    return 1 + check(node->left) + check(node->right);
}

static void release(struct node *node) {
    if (node->left) release(node->left);
    if (node->right) release(node->right);
    free(node);
}

int main(int argc, char **argv) {
    int n = argc > 1 ? atoi(argv[1]) : 10;
    int max_depth = n > 6 ? n : 6, stretch = max_depth + 1;
    struct node *tree = make(stretch);
    printf("stretch tree of depth %d\t check: %ld\n", stretch, check(tree));
    release(tree);
    struct node *long_lived = make(max_depth);
    for (int depth = 4; depth <= max_depth; depth += 2) {
        long iterations = 1L << (max_depth - depth + 4), total = 0;
        for (long i = 0; i < iterations; i++) {
            tree = make(depth);
            total += check(tree);
            release(tree);
        }
        printf("%ld\t trees of depth %d\t check: %ld\n", iterations, depth, total);
    }
    printf("long lived tree of depth %d\t check: %ld\n", max_depth, check(long_lived));
    release(long_lived);
    return 0;
}
