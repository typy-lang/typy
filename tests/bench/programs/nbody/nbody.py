"""The n-body simulation of the Benchmarks Game: five bodies, advanced N steps."""

import math
import sys

PI = 3.141592653589793
SOLAR_MASS = 4.0 * PI * PI
DAYS_PER_YEAR = 365.24


class Body:
    def __init__(
        self, x: float, y: float, z: float, vx: float, vy: float, vz: float, mass: float
    ) -> None:
        self.x = x
        self.y = y
        self.z = z
        self.vx = vx * DAYS_PER_YEAR
        self.vy = vy * DAYS_PER_YEAR
        self.vz = vz * DAYS_PER_YEAR
        self.mass = mass * SOLAR_MASS


def make_bodies() -> list[Body]:
    return [
        Body(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
        Body(
            4.84143144246472090e00,
            -1.16032004402742839e00,
            -1.03622044471123109e-01,
            1.66007664274403694e-03,
            7.69901118419740425e-03,
            -6.90460016972063023e-05,
            9.54791938424326609e-04,
        ),
        Body(
            8.34336671824457987e00,
            4.12479856412430479e00,
            -4.03523417114321381e-01,
            -2.76742510726862411e-03,
            4.99852801234917238e-03,
            2.30417297573763929e-05,
            2.85885980666130812e-04,
        ),
        Body(
            1.28943695621391310e01,
            -1.51111514016986312e01,
            -2.23307578892655734e-01,
            2.96460137564761618e-03,
            2.37847173959480950e-03,
            -2.96589568540237556e-05,
            4.36624404335156298e-05,
        ),
        Body(
            1.53796971148509165e01,
            -2.59193146099879641e01,
            1.79258772950371181e-01,
            2.68067772490389322e-03,
            1.62824170038242295e-03,
            -9.51592254519715870e-05,
            5.15138902046611451e-05,
        ),
    ]


def offset_momentum(bodies: list[Body]) -> None:
    px = 0.0
    py = 0.0
    pz = 0.0
    for body in bodies:
        px += body.vx * body.mass
        py += body.vy * body.mass
        pz += body.vz * body.mass
    sun = bodies[0]
    sun.vx = -px / SOLAR_MASS
    sun.vy = -py / SOLAR_MASS
    sun.vz = -pz / SOLAR_MASS


def energy(bodies: list[Body]) -> float:
    total = 0.0
    count = len(bodies)
    for i in range(count):
        body = bodies[i]
        total += 0.5 * body.mass * (body.vx * body.vx + body.vy * body.vy + body.vz * body.vz)
        for j in range(i + 1, count):
            other = bodies[j]
            dx = body.x - other.x
            dy = body.y - other.y
            dz = body.z - other.z
            total -= body.mass * other.mass / math.sqrt(dx * dx + dy * dy + dz * dz)
    return total


def advance(bodies: list[Body], dt: float, steps: int) -> None:
    count = len(bodies)
    for _ in range(steps):
        for i in range(count):
            body = bodies[i]
            for j in range(i + 1, count):
                other = bodies[j]
                dx = body.x - other.x
                dy = body.y - other.y
                dz = body.z - other.z
                distance2 = dx * dx + dy * dy + dz * dz
                magnitude = dt / (distance2 * math.sqrt(distance2))
                body.vx -= dx * other.mass * magnitude
                body.vy -= dy * other.mass * magnitude
                body.vz -= dz * other.mass * magnitude
                other.vx += dx * body.mass * magnitude
                other.vy += dy * body.mass * magnitude
                other.vz += dz * body.mass * magnitude
        for body in bodies:
            body.x += dt * body.vx
            body.y += dt * body.vy
            body.z += dt * body.vz


def main() -> None:
    steps = int(sys.argv[1])
    bodies = make_bodies()
    offset_momentum(bodies)
    print(f"{energy(bodies):.9f}")
    advance(bodies, 0.01, steps)
    print(f"{energy(bodies):.9f}")


main()
