// The Mandelbrot set of the Benchmarks Game, as the count of points of an N by N grid in it.
"use strict";

const n = Number(process.argv[2]);
let found = 0;
for (let y = 0; y < n; y++) {
  const ci = (2.0 * y) / n - 1.0;
  for (let x = 0; x < n; x++) {
    const cr = (2.0 * x) / n - 1.5;
    let zr = 0.0, zi = 0.0, escaped = false;
    for (let i = 0; i < 50; i++) {
      const nextR = zr * zr - zi * zi + cr;
      zi = 2.0 * zr * zi + ci;
      zr = nextR;
      if (zr * zr + zi * zi > 4.0) {
        escaped = true;
        break;
      }
    }
    if (!escaped) found++;
  }
}
console.log(found);
