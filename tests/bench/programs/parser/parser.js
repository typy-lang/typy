// A small interpreter: arithmetic expressions written out, tokenized and evaluated.
"use strict";

const MODULUS = 1000003;

// Python's `%`, whose result takes the divisor's sign.
function modulo(a, m) {
  return ((a % m) + m) % m;
}

function expression(seed) {
  const parts = [];
  for (let k = 0; k < 12; k++) {
    const value = String(((seed * 31 + k * 17) % 97) + 1);
    parts.push(k % 4 === 0 ? "(" + value : k % 4 === 3 ? value + ")" : value);
  }
  const operators = ["+", "*", "-", "+"];
  let text = parts[0];
  for (let k = 1; k < 12; k++) text += " " + operators[(seed + k) % 4] + " " + parts[k];
  return text;
}

function tokens(text) {
  const found = [];
  let number = "";
  for (const c of text) {
    if (c >= "0" && c <= "9") {
      number += c;
      continue;
    }
    if (number) {
      found.push(number);
      number = "";
    }
    if (c !== " ") found.push(c);
  }
  if (number) found.push(number);
  return found;
}

class Parser {
  constructor(items) {
    this.items = items;
    this.position = 0;
  }
  peek() {
    return this.position < this.items.length ? this.items[this.position] : "";
  }
  take() {
    return this.items[this.position++];
  }
  sum() {
    let value = this.product();
    while (this.peek() === "+" || this.peek() === "-") {
      if (this.take() === "+") value = modulo(value + this.product(), MODULUS);
      else value = modulo(value - this.product(), MODULUS);
    }
    return value;
  }
  product() {
    let value = this.atom();
    while (this.peek() === "*") {
      this.take();
      value = modulo(value * this.atom(), MODULUS);
    }
    return value;
  }
  atom() {
    const item = this.take();
    if (item === "(") {
      const value = this.sum();
      this.take();
      return value;
    }
    return Number(item);
  }
}

const n = Number(process.argv[2]);
let total = 0;
for (let i = 0; i < n; i++) total = modulo(total + new Parser(tokens(expression(i))).sum(), MODULUS);
console.log(total);
