// The binary trees of the Benchmarks Game: many short-lived trees, allocated and freed.
"use strict";

class Node {
  constructor(left, right) {
    this.left = left;
    this.right = right;
  }
}

function make(depth) {
  return depth === 0 ? new Node(null, null) : new Node(make(depth - 1), make(depth - 1));
}

function check(node) {
  if (node.left === null || node.right === null) return 1;
  return 1 + check(node.left) + check(node.right);
}

const maxDepth = Math.max(6, Number(process.argv[2]));
const stretch = maxDepth + 1;
console.log(`stretch tree of depth ${stretch}\t check: ${check(make(stretch))}`);
const longLived = make(maxDepth);
for (let depth = 4; depth <= maxDepth; depth += 2) {
  const iterations = 1 << (maxDepth - depth + 4);
  let total = 0;
  for (let i = 0; i < iterations; i++) total += check(make(depth));
  console.log(`${iterations}\t trees of depth ${depth}\t check: ${total}`);
}
console.log(`long lived tree of depth ${maxDepth}\t check: ${check(longLived)}`);
