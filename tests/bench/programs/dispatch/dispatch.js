// Method calls through a base class, as an object-oriented program makes them.
"use strict";

class Shape {
  area() {
    return 0.0;
  }
}

class Square extends Shape {
  constructor(side) {
    super();
    this.side = side;
  }
  area() {
    return this.side * this.side;
  }
}

class Circle extends Shape {
  constructor(radius) {
    super();
    this.radius = radius;
  }
  area() {
    return 3.14159 * this.radius * this.radius;
  }
}

class Rectangle extends Shape {
  constructor(width, height) {
    super();
    this.width = width;
    this.height = height;
  }
  area() {
    return this.width * this.height;
  }
}

const rounds = Number(process.argv[2]);
const made = [];
for (let i = 0; i < 1000; i++) {
  const size = (i % 7) + 1;
  if (i % 3 === 0) made.push(new Square(size));
  else if (i % 3 === 1) made.push(new Circle(size));
  else made.push(new Rectangle(size, size + 1.0));
}
let found = 0.0;
for (let r = 0; r < rounds; r++) for (const shape of made) found += shape.area();
console.log(found.toFixed(3));
