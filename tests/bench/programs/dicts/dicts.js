// Dictionary work as programs do it: counting words, then ranking the most frequent.
"use strict";

const n = Number(process.argv[2]);
const found = new Map();
let seed = 1;
for (let i = 0; i < n; i++) {
  // The low 31 bits of the product, which a double would round: `Math.imul` keeps them.
  seed = (Math.imul(seed, 1103515245) + 12345) & 0x7fffffff;
  const word = "w" + (seed % 5000);
  found.set(word, (found.get(word) || 0) + 1);
}
const ranked = [...found].sort((a, b) =>
  a[1] !== b[1] ? b[1] - a[1] : a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0);
const out = [String(found.size)];
for (const [word, count] of ranked.slice(0, 5)) out.push(`${word} ${count}`);
console.log(out.join("\n"));
