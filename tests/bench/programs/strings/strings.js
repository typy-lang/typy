// String work as programs do it: joining, splitting, case, searching and replacing.
"use strict";

const WORDS = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"];

function stripped(line, c) {
  let start = 0, end = line.length;
  while (start < end && line[start] === c) start++;
  while (end > start && line[end - 1] === c) end--;
  return line.slice(start, end);
}

function work(n) {
  let total = 0;
  for (let i = 0; i < n; i++) {
    const parts = [];
    for (let k = 0; k < 8; k++) parts.push(WORDS[(i + k) % WORDS.length]);
    const line = parts.join(" ");
    const upper = line.toUpperCase();
    const pieces = upper.split(" ");
    total += pieces.length + upper.indexOf("GAMMA") + line.split("a").length - 1;
    total += line.replaceAll("beta", "b").length + stripped(line, "a").length;
    if (line.startsWith("alpha")) total++;
  }
  return total;
}

console.log(work(Number(process.argv[2])));
