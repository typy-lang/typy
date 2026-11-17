// The fannkuch-redux of the Benchmarks Game: flips of every permutation of N items.
"use strict";

function fannkuch(n) {
  const first = Int32Array.from({ length: n }, (_, i) => i);
  const perm = new Int32Array(n);
  const count = new Int32Array(n);
  let maxFlips = 0, checksum = 0, permutations = 0, r = n;
  for (;;) {
    for (; r !== 1; r--) count[r - 1] = r;
    perm.set(first);
    let flips = 0;
    for (let k = perm[0]; k !== 0; k = perm[0]) {
      for (let i = 0, j = k; i < j; i++, j--) {
        const held = perm[i];
        perm[i] = perm[j];
        perm[j] = held;
      }
      flips++;
    }
    if (flips > maxFlips) maxFlips = flips;
    checksum += permutations % 2 === 0 ? flips : -flips;
    for (;;) {
      if (r === n) return [checksum, maxFlips];
      const head = first[0];
      first.copyWithin(0, 1, r + 1);
      first[r] = head;
      if (--count[r] > 0) break;
      r++;
    }
    permutations++;
  }
}

const n = Number(process.argv[2]);
const [checksum, maxFlips] = fannkuch(n);
console.log(`${checksum}\nPfannkuchen(${n}) = ${maxFlips}`);
