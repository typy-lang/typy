// The spectral norm of the Benchmarks Game: the largest eigenvalue of an infinite matrix.
"use strict";

function element(i, j) {
  return 1.0 / (((i + j) * (i + j + 1)) / 2 + i + 1);
}

function times(u, v) {
  for (let i = 0; i < v.length; i++) {
    let total = 0.0;
    for (let j = 0; j < u.length; j++) total += element(i, j) * u[j];
    v[i] = total;
  }
}

function timesTransposed(u, v) {
  for (let i = 0; i < v.length; i++) {
    let total = 0.0;
    for (let j = 0; j < u.length; j++) total += element(j, i) * u[j];
    v[i] = total;
  }
}

const n = Number(process.argv[2]);
const u = new Float64Array(n).fill(1.0);
const v = new Float64Array(n);
const scratch = new Float64Array(n);
for (let k = 0; k < 10; k++) {
  times(u, scratch);
  timesTransposed(scratch, v);
  times(v, scratch);
  timesTransposed(scratch, u);
}
let vbv = 0.0, vv = 0.0;
for (let i = 0; i < n; i++) {
  vbv += u[i] * v[i];
  vv += v[i] * v[i];
}
console.log(Math.sqrt(vbv / vv).toFixed(9));
