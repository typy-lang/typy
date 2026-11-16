// The n-body simulation of the Benchmarks Game: five bodies, advanced N steps.
"use strict";

const PI = 3.141592653589793;
const SOLAR_MASS = 4.0 * PI * PI;
const DAYS_PER_YEAR = 365.24;

class Body {
  constructor(x, y, z, vx, vy, vz, mass) {
    this.x = x;
    this.y = y;
    this.z = z;
    this.vx = vx * DAYS_PER_YEAR;
    this.vy = vy * DAYS_PER_YEAR;
    this.vz = vz * DAYS_PER_YEAR;
    this.mass = mass * SOLAR_MASS;
  }
}

function offsetMomentum(bodies) {
  let px = 0.0, py = 0.0, pz = 0.0;
  for (const b of bodies) {
    px += b.vx * b.mass;
    py += b.vy * b.mass;
    pz += b.vz * b.mass;
  }
  bodies[0].vx = -px / SOLAR_MASS;
  bodies[0].vy = -py / SOLAR_MASS;
  bodies[0].vz = -pz / SOLAR_MASS;
}

function energy(bodies) {
  let total = 0.0;
  for (let i = 0; i < bodies.length; i++) {
    const b = bodies[i];
    total += 0.5 * b.mass * (b.vx * b.vx + b.vy * b.vy + b.vz * b.vz);
    for (let j = i + 1; j < bodies.length; j++) {
      const o = bodies[j];
      const dx = b.x - o.x, dy = b.y - o.y, dz = b.z - o.z;
      total -= (b.mass * o.mass) / Math.sqrt(dx * dx + dy * dy + dz * dz);
    }
  }
  return total;
}

function advance(bodies, dt, steps) {
  for (let step = 0; step < steps; step++) {
    for (let i = 0; i < bodies.length; i++) {
      const b = bodies[i];
      for (let j = i + 1; j < bodies.length; j++) {
        const o = bodies[j];
        const dx = b.x - o.x, dy = b.y - o.y, dz = b.z - o.z;
        const distance2 = dx * dx + dy * dy + dz * dz;
        const magnitude = dt / (distance2 * Math.sqrt(distance2));
        b.vx -= dx * o.mass * magnitude;
        b.vy -= dy * o.mass * magnitude;
        b.vz -= dz * o.mass * magnitude;
        o.vx += dx * b.mass * magnitude;
        o.vy += dy * b.mass * magnitude;
        o.vz += dz * b.mass * magnitude;
      }
    }
    for (const b of bodies) {
      b.x += dt * b.vx;
      b.y += dt * b.vy;
      b.z += dt * b.vz;
    }
  }
}

const steps = Number(process.argv[2]);
const bodies = [
  new Body(0, 0, 0, 0, 0, 0, 1),
  new Body(4.8414314424647209, -1.16032004402742839, -1.03622044471123109e-1,
    1.66007664274403694e-3, 7.69901118419740425e-3, -6.90460016972063023e-5,
    9.54791938424326609e-4),
  new Body(8.34336671824457987, 4.12479856412430479, -4.03523417114321381e-1,
    -2.76742510726862411e-3, 4.99852801234917238e-3, 2.30417297573763929e-5,
    2.85885980666130812e-4),
  new Body(1.2894369562139131e1, -1.51111514016986312e1, -2.23307578892655734e-1,
    2.96460137564761618e-3, 2.3784717395948095e-3, -2.96589568540237556e-5,
    4.36624404335156298e-5),
  new Body(1.53796971148509165e1, -2.59193146099879641e1, 1.79258772950371181e-1,
    2.68067772490389322e-3, 1.62824170038242295e-3, -9.5159225451971587e-5,
    5.15138902046611451e-5),
];
offsetMomentum(bodies);
console.log(energy(bodies).toFixed(9));
advance(bodies, 0.01, steps);
console.log(energy(bodies).toFixed(9));
