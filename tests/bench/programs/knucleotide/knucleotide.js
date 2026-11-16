// The k-nucleotide of the Benchmarks Game, over a sequence of N drawn as fasta draws it.
"use strict";

const IM = 139968, IA = 3877, IC = 29573;

function sequence(n) {
  const homoSapiens = [0.302954942668, 0.1979883004921, 0.1975473066391, 0.3015094502008];
  const table = [];
  let total = 0.0;
  for (const p of homoSapiens) table.push((total += p));
  table[3] = 1.0;
  const characters = [];
  let seed = 42;
  for (let i = 0; i < n; i++) {
    seed = (seed * IA + IC) % IM;
    const drawn = (1.0 * seed) / IM;
    let k = 0;
    while (drawn >= table[k]) k++;
    characters.push("ACGT"[k]);
  }
  return characters.join("");
}

function counts(text, k) {
  const found = new Map();
  for (let i = 0; i + k <= text.length; i++) {
    const fragment = text.slice(i, i + k);
    found.set(fragment, (found.get(fragment) || 0) + 1);
  }
  return found;
}

const text = sequence(Number(process.argv[2]));
const out = [];
for (let k = 1; k <= 2; k++) {
  const ranked = [...counts(text, k)].sort((a, b) =>
    a[1] !== b[1] ? b[1] - a[1] : a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0);
  const total = text.length - k + 1;
  for (const [fragment, count] of ranked) out.push(`${fragment} ${((100.0 * count) / total).toFixed(3)}`);
  out.push("");
}
for (const fragment of ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]) {
  out.push(`${counts(text, fragment.length).get(fragment) || 0}\t${fragment}`);
}
console.log(out.join("\n"));
