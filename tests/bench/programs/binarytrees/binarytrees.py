"""The binary trees of the Benchmarks Game: many short-lived trees, allocated and freed."""

import sys


class Node:
    def __init__(self, left: "Node | None", right: "Node | None") -> None:
        self.left = left
        self.right = right


def make(depth: int) -> Node:
    if depth == 0:
        return Node(None, None)
    return Node(make(depth - 1), make(depth - 1))


def check(node: Node) -> int:
    left = node.left
    right = node.right
    if left is None:
        return 1
    if right is None:
        return 1
    return 1 + check(left) + check(right)


def main() -> None:
    n = int(sys.argv[1])
    max_depth = max(6, n)
    stretch = max_depth + 1
    print(f"stretch tree of depth {stretch}\t check: {check(make(stretch))}")
    long_lived = make(max_depth)
    for depth in range(4, max_depth + 1, 2):
        iterations = 1 << (max_depth - depth + 4)
        total = 0
        for _ in range(iterations):
            total += check(make(depth))
        print(f"{iterations}\t trees of depth {depth}\t check: {total}")
    print(f"long lived tree of depth {max_depth}\t check: {check(long_lived)}")


main()
