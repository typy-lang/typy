// The n-body simulation of the Benchmarks Game: five bodies, advanced N steps.
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <vector>

constexpr double PI = 3.141592653589793;
constexpr double SOLAR_MASS = 4.0 * PI * PI;
constexpr double DAYS_PER_YEAR = 365.24;

struct Body {
    double x, y, z, vx, vy, vz, mass;
    Body(double x, double y, double z, double vx, double vy, double vz, double mass)
        : x(x), y(y), z(z), vx(vx * DAYS_PER_YEAR), vy(vy * DAYS_PER_YEAR), vz(vz * DAYS_PER_YEAR),
          mass(mass * SOLAR_MASS) {}
};

static void offset_momentum(std::vector<Body> &bodies) {
    double px = 0.0, py = 0.0, pz = 0.0;
    for (const Body &b : bodies) {
        px += b.vx * b.mass;
        py += b.vy * b.mass;
        pz += b.vz * b.mass;
    }
    bodies[0].vx = -px / SOLAR_MASS;
    bodies[0].vy = -py / SOLAR_MASS;
    bodies[0].vz = -pz / SOLAR_MASS;
}

static double energy(const std::vector<Body> &bodies) {
    double total = 0.0;
    for (size_t i = 0; i < bodies.size(); i++) {
        const Body &b = bodies[i];
        total += 0.5 * b.mass * (b.vx * b.vx + b.vy * b.vy + b.vz * b.vz);
        for (size_t j = i + 1; j < bodies.size(); j++) {
            const Body &o = bodies[j];
            double dx = b.x - o.x, dy = b.y - o.y, dz = b.z - o.z;
            total -= b.mass * o.mass / std::sqrt(dx * dx + dy * dy + dz * dz);
        }
    }
    return total;
}

static void advance(std::vector<Body> &bodies, double dt, long steps) {
    for (long step = 0; step < steps; step++) {
        for (size_t i = 0; i < bodies.size(); i++) {
            Body &b = bodies[i];
            for (size_t j = i + 1; j < bodies.size(); j++) {
                Body &o = bodies[j];
                double dx = b.x - o.x, dy = b.y - o.y, dz = b.z - o.z;
                double distance2 = dx * dx + dy * dy + dz * dz;
                double magnitude = dt / (distance2 * std::sqrt(distance2));
                b.vx -= dx * o.mass * magnitude;
                b.vy -= dy * o.mass * magnitude;
                b.vz -= dz * o.mass * magnitude;
                o.vx += dx * b.mass * magnitude;
                o.vy += dy * b.mass * magnitude;
                o.vz += dz * b.mass * magnitude;
            }
        }
        for (Body &b : bodies) {
            b.x += dt * b.vx;
            b.y += dt * b.vy;
            b.z += dt * b.vz;
        }
    }
}

int main(int argc, char **argv) {
    long steps = argc > 1 ? std::atol(argv[1]) : 1000;
    std::vector<Body> bodies{
        {0, 0, 0, 0, 0, 0, 1},
        {4.84143144246472090e00, -1.16032004402742839e00, -1.03622044471123109e-01,
         1.66007664274403694e-03, 7.69901118419740425e-03, -6.90460016972063023e-05,
         9.54791938424326609e-04},
        {8.34336671824457987e00, 4.12479856412430479e00, -4.03523417114321381e-01,
         -2.76742510726862411e-03, 4.99852801234917238e-03, 2.30417297573763929e-05,
         2.85885980666130812e-04},
        {1.28943695621391310e01, -1.51111514016986312e01, -2.23307578892655734e-01,
         2.96460137564761618e-03, 2.37847173959480950e-03, -2.96589568540237556e-05,
         4.36624404335156298e-05},
        {1.53796971148509165e01, -2.59193146099879641e01, 1.79258772950371181e-01,
         2.68067772490389322e-03, 1.62824170038242295e-03, -9.51592254519715870e-05,
         5.15138902046611451e-05},
    };
    offset_momentum(bodies);
    std::printf("%.9f\n", energy(bodies));
    advance(bodies, 0.01, steps);
    std::printf("%.9f\n", energy(bodies));
}
