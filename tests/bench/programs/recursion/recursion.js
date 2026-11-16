// Recursive calls: the Fibonacci numbers by their definition.
"use strict";

function fibonacci(n) {
  return n < 2 ? n : fibonacci(n - 1) + fibonacci(n - 2);
}

console.log(fibonacci(Number(process.argv[2])));
