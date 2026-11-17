// The n-body simulation of the Benchmarks Game: five bodies, advanced N steps.
package main

import (
	"fmt"
	"math"
	"os"
	"strconv"
)

const (
	pi          = 3.141592653589793
	solarMass   = 4.0 * pi * pi
	daysPerYear = 365.24
)

type body struct{ x, y, z, vx, vy, vz, mass float64 }

func newBody(x, y, z, vx, vy, vz, mass float64) body {
	return body{x, y, z, vx * daysPerYear, vy * daysPerYear, vz * daysPerYear, mass * solarMass}
}

func offsetMomentum(bodies []body) {
	var px, py, pz float64
	for _, b := range bodies {
		px += b.vx * b.mass
		py += b.vy * b.mass
		pz += b.vz * b.mass
	}
	bodies[0].vx = -px / solarMass
	bodies[0].vy = -py / solarMass
	bodies[0].vz = -pz / solarMass
}

func energy(bodies []body) float64 {
	total := 0.0
	for i := range bodies {
		b := &bodies[i]
		total += 0.5 * b.mass * (b.vx*b.vx + b.vy*b.vy + b.vz*b.vz)
		for j := i + 1; j < len(bodies); j++ {
			o := &bodies[j]
			dx, dy, dz := b.x-o.x, b.y-o.y, b.z-o.z
			total -= b.mass * o.mass / math.Sqrt(dx*dx+dy*dy+dz*dz)
		}
	}
	return total
}

func advance(bodies []body, dt float64, steps int) {
	for step := 0; step < steps; step++ {
		for i := range bodies {
			b := &bodies[i]
			for j := i + 1; j < len(bodies); j++ {
				o := &bodies[j]
				dx, dy, dz := b.x-o.x, b.y-o.y, b.z-o.z
				distance2 := dx*dx + dy*dy + dz*dz
				magnitude := dt / (distance2 * math.Sqrt(distance2))
				b.vx -= dx * o.mass * magnitude
				b.vy -= dy * o.mass * magnitude
				b.vz -= dz * o.mass * magnitude
				o.vx += dx * b.mass * magnitude
				o.vy += dy * b.mass * magnitude
				o.vz += dz * b.mass * magnitude
			}
		}
		for i := range bodies {
			b := &bodies[i]
			b.x += dt * b.vx
			b.y += dt * b.vy
			b.z += dt * b.vz
		}
	}
}

func main() {
	steps, _ := strconv.Atoi(os.Args[1])
	bodies := []body{
		newBody(0, 0, 0, 0, 0, 0, 1),
		newBody(4.84143144246472090e00, -1.16032004402742839e00, -1.03622044471123109e-01,
			1.66007664274403694e-03, 7.69901118419740425e-03, -6.90460016972063023e-05,
			9.54791938424326609e-04),
		newBody(8.34336671824457987e00, 4.12479856412430479e00, -4.03523417114321381e-01,
			-2.76742510726862411e-03, 4.99852801234917238e-03, 2.30417297573763929e-05,
			2.85885980666130812e-04),
		newBody(1.28943695621391310e01, -1.51111514016986312e01, -2.23307578892655734e-01,
			2.96460137564761618e-03, 2.37847173959480950e-03, -2.96589568540237556e-05,
			4.36624404335156298e-05),
		newBody(1.53796971148509165e01, -2.59193146099879641e01, 1.79258772950371181e-01,
			2.68067772490389322e-03, 1.62824170038242295e-03, -9.51592254519715870e-05,
			5.15138902046611451e-05),
	}
	offsetMomentum(bodies)
	fmt.Printf("%.9f\n", energy(bodies))
	advance(bodies, 0.01, steps)
	fmt.Printf("%.9f\n", energy(bodies))
}
