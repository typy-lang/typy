//! The binary trees of the Benchmarks Game: many short-lived trees, allocated and freed.

struct Node {
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

fn make(depth: u32) -> Box<Node> {
    if depth == 0 {
        return Box::new(Node { left: None, right: None });
    }
    Box::new(Node { left: Some(make(depth - 1)), right: Some(make(depth - 1)) })
}

fn check(node: &Node) -> i64 {
    match (&node.left, &node.right) {
        (Some(left), Some(right)) => 1 + check(left) + check(right),
        _ => 1,
    }
}

fn main() {
    let n: u32 = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(10);
    let max_depth = n.max(6);
    let stretch = max_depth + 1;
    println!("stretch tree of depth {stretch}\t check: {}", check(&make(stretch)));
    let long_lived = make(max_depth);
    for depth in (4..=max_depth).step_by(2) {
        let iterations = 1i64 << (max_depth - depth + 4);
        let total: i64 = (0..iterations).map(|_| check(&make(depth))).sum();
        println!("{iterations}\t trees of depth {depth}\t check: {total}");
    }
    println!("long lived tree of depth {max_depth}\t check: {}", check(&long_lived));
}
