//! The n-body simulation of the Benchmarks Game: five bodies, advanced N steps.

const PI: f64 = 3.141592653589793;
const SOLAR_MASS: f64 = 4.0 * PI * PI;
const DAYS_PER_YEAR: f64 = 365.24;

#[derive(Clone, Copy)]
struct Body {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    mass: f64,
}

impl Body {
    fn new(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64, mass: f64) -> Self {
        Self {
            x,
            y,
            z,
            vx: vx * DAYS_PER_YEAR,
            vy: vy * DAYS_PER_YEAR,
            vz: vz * DAYS_PER_YEAR,
            mass: mass * SOLAR_MASS,
        }
    }
}

fn offset_momentum(bodies: &mut [Body]) {
    let (mut px, mut py, mut pz) = (0.0, 0.0, 0.0);
    for body in bodies.iter() {
        px += body.vx * body.mass;
        py += body.vy * body.mass;
        pz += body.vz * body.mass;
    }
    bodies[0].vx = -px / SOLAR_MASS;
    bodies[0].vy = -py / SOLAR_MASS;
    bodies[0].vz = -pz / SOLAR_MASS;
}

fn energy(bodies: &[Body]) -> f64 {
    let mut total = 0.0;
    for (i, body) in bodies.iter().enumerate() {
        total += 0.5 * body.mass * (body.vx * body.vx + body.vy * body.vy + body.vz * body.vz);
        for other in &bodies[i + 1..] {
            let (dx, dy, dz) = (body.x - other.x, body.y - other.y, body.z - other.z);
            total -= body.mass * other.mass / (dx * dx + dy * dy + dz * dz).sqrt();
        }
    }
    total
}

fn advance(bodies: &mut [Body], dt: f64, steps: usize) {
    let count = bodies.len();
    for _ in 0..steps {
        for i in 0..count {
            let (left, right) = bodies.split_at_mut(i + 1);
            let body = &mut left[i];
            for other in right.iter_mut() {
                let (dx, dy, dz) = (body.x - other.x, body.y - other.y, body.z - other.z);
                let distance2 = dx * dx + dy * dy + dz * dz;
                let magnitude = dt / (distance2 * distance2.sqrt());
                body.vx -= dx * other.mass * magnitude;
                body.vy -= dy * other.mass * magnitude;
                body.vz -= dz * other.mass * magnitude;
                other.vx += dx * body.mass * magnitude;
                other.vy += dy * body.mass * magnitude;
                other.vz += dz * body.mass * magnitude;
            }
        }
        for body in bodies.iter_mut() {
            body.x += dt * body.vx;
            body.y += dt * body.vy;
            body.z += dt * body.vz;
        }
    }
}

fn main() {
    let steps: usize = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(1000);
    let mut bodies = [
        Body::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
        Body::new(
            4.84143144246472090e00,
            -1.16032004402742839e00,
            -1.03622044471123109e-01,
            1.66007664274403694e-03,
            7.69901118419740425e-03,
            -6.90460016972063023e-05,
            9.54791938424326609e-04,
        ),
        Body::new(
            8.34336671824457987e00,
            4.12479856412430479e00,
            -4.03523417114321381e-01,
            -2.76742510726862411e-03,
            4.99852801234917238e-03,
            2.30417297573763929e-05,
            2.85885980666130812e-04,
        ),
        Body::new(
            1.28943695621391310e01,
            -1.51111514016986312e01,
            -2.23307578892655734e-01,
            2.96460137564761618e-03,
            2.37847173959480950e-03,
            -2.96589568540237556e-05,
            4.36624404335156298e-05,
        ),
        Body::new(
            1.53796971148509165e01,
            -2.59193146099879641e01,
            1.79258772950371181e-01,
            2.68067772490389322e-03,
            1.62824170038242295e-03,
            -9.51592254519715870e-05,
            5.15138902046611451e-05,
        ),
    ];
    offset_momentum(&mut bodies);
    println!("{:.9}", energy(&bodies));
    advance(&mut bodies, 0.01, steps);
    println!("{:.9}", energy(&bodies));
}
