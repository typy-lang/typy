// The binary trees of the Benchmarks Game: many short-lived trees, allocated and freed.
public final class BinaryTrees {
    record Node(Node left, Node right) {}

    static Node make(int depth) {
        return depth == 0 ? new Node(null, null) : new Node(make(depth - 1), make(depth - 1));
    }

    static long check(Node node) {
        if (node.left() == null || node.right() == null) return 1;
        return 1 + check(node.left()) + check(node.right());
    }

    public static void main(String[] args) {
        int maxDepth = Math.max(6, Integer.parseInt(args[0]));
        int stretch = maxDepth + 1;
        System.out.println("stretch tree of depth " + stretch + "\t check: " + check(make(stretch)));
        Node longLived = make(maxDepth);
        for (int depth = 4; depth <= maxDepth; depth += 2) {
            long iterations = 1L << (maxDepth - depth + 4), total = 0;
            for (long i = 0; i < iterations; i++) total += check(make(depth));
            System.out.println(iterations + "\t trees of depth " + depth + "\t check: " + total);
        }
        System.out.println("long lived tree of depth " + maxDepth + "\t check: " + check(longLived));
    }
}
