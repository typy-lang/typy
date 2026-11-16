/* The n-body simulation of the Benchmarks Game: five bodies, advanced N steps. */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

#define PI 3.141592653589793
#define SOLAR_MASS (4.0 * PI * PI)
#define DAYS_PER_YEAR 365.24
#define COUNT 5

struct body {
    double x, y, z, vx, vy, vz, mass;
};

static void make(struct body *b, double x, double y, double z, double vx, double vy, double vz,
                 double mass) {
    b->x = x;
    b->y = y;
    b->z = z;
    b->vx = vx * DAYS_PER_YEAR;
    b->vy = vy * DAYS_PER_YEAR;
    b->vz = vz * DAYS_PER_YEAR;
    b->mass = mass * SOLAR_MASS;
}

static void offset_momentum(struct body *bodies) {
    double px = 0.0, py = 0.0, pz = 0.0;
    for (int i = 0; i < COUNT; i++) {
        px += bodies[i].vx * bodies[i].mass;
        py += bodies[i].vy * bodies[i].mass;
        pz += bodies[i].vz * bodies[i].mass;
    }
    bodies[0].vx = -px / SOLAR_MASS;
    bodies[0].vy = -py / SOLAR_MASS;
    bodies[0].vz = -pz / SOLAR_MASS;
}

static double energy(const struct body *bodies) {
    double total = 0.0;
    for (int i = 0; i < COUNT; i++) {
        const struct body *b = &bodies[i];
        total += 0.5 * b->mass * (b->vx * b->vx + b->vy * b->vy + b->vz * b->vz);
        for (int j = i + 1; j < COUNT; j++) {
            const struct body *o = &bodies[j];
            double dx = b->x - o->x, dy = b->y - o->y, dz = b->z - o->z;
            total -= b->mass * o->mass / sqrt(dx * dx + dy * dy + dz * dz);
        }
    }
    return total;
}

static void advance(struct body *bodies, double dt, long steps) {
    for (long step = 0; step < steps; step++) {
        for (int i = 0; i < COUNT; i++) {
            struct body *b = &bodies[i];
            for (int j = i + 1; j < COUNT; j++) {
                struct body *o = &bodies[j];
                double dx = b->x - o->x, dy = b->y - o->y, dz = b->z - o->z;
                double distance2 = dx * dx + dy * dy + dz * dz;
                double magnitude = dt / (distance2 * sqrt(distance2));
                b->vx -= dx * o->mass * magnitude;
                b->vy -= dy * o->mass * magnitude;
                b->vz -= dz * o->mass * magnitude;
                o->vx += dx * b->mass * magnitude;
                o->vy += dy * b->mass * magnitude;
                o->vz += dz * b->mass * magnitude;
            }
        }
        for (int i = 0; i < COUNT; i++) {
            bodies[i].x += dt * bodies[i].vx;
            bodies[i].y += dt * bodies[i].vy;
            bodies[i].z += dt * bodies[i].vz;
        }
    }
}

int main(int argc, char **argv) {
    long steps = argc > 1 ? atol(argv[1]) : 1000;
    struct body bodies[COUNT];
    make(&bodies[0], 0, 0, 0, 0, 0, 0, 1);
    make(&bodies[1], 4.84143144246472090e00, -1.16032004402742839e00, -1.03622044471123109e-01,
         1.66007664274403694e-03, 7.69901118419740425e-03, -6.90460016972063023e-05,
         9.54791938424326609e-04);
    make(&bodies[2], 8.34336671824457987e00, 4.12479856412430479e00, -4.03523417114321381e-01,
         -2.76742510726862411e-03, 4.99852801234917238e-03, 2.30417297573763929e-05,
         2.85885980666130812e-04);
    make(&bodies[3], 1.28943695621391310e01, -1.51111514016986312e01, -2.23307578892655734e-01,
         2.96460137564761618e-03, 2.37847173959480950e-03, -2.96589568540237556e-05,
         4.36624404335156298e-05);
    make(&bodies[4], 1.53796971148509165e01, -2.59193146099879641e01, 1.79258772950371181e-01,
         2.68067772490389322e-03, 1.62824170038242295e-03, -9.51592254519715870e-05,
         5.15138902046611451e-05);
    offset_momentum(bodies);
    printf("%.9f\n", energy(bodies));
    advance(bodies, 0.01, steps);
    printf("%.9f\n", energy(bodies));
    return 0;
}
