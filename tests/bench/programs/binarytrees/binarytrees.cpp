// The binary trees of the Benchmarks Game: many short-lived trees, allocated and freed.
#include <cstdio>
#include <cstdlib>
#include <memory>

struct Node {
    std::unique_ptr<Node> left, right;
};

static std::unique_ptr<Node> make(int depth) {
    auto node = std::make_unique<Node>();
    if (depth > 0) {
        node->left = make(depth - 1);
        node->right = make(depth - 1);
    }
    return node;
}

static long check(const Node &node) {
    if (!node.left || !node.right) return 1;
    return 1 + check(*node.left) + check(*node.right);
}

int main(int argc, char **argv) {
    int n = argc > 1 ? std::atoi(argv[1]) : 10;
    int max_depth = n > 6 ? n : 6, stretch = max_depth + 1;
    std::printf("stretch tree of depth %d\t check: %ld\n", stretch, check(*make(stretch)));
    auto long_lived = make(max_depth);
    for (int depth = 4; depth <= max_depth; depth += 2) {
        long iterations = 1L << (max_depth - depth + 4), total = 0;
        for (long i = 0; i < iterations; i++) total += check(*make(depth));
        std::printf("%ld\t trees of depth %d\t check: %ld\n", iterations, depth, total);
    }
    std::printf("long lived tree of depth %d\t check: %ld\n", max_depth, check(*long_lived));
}
