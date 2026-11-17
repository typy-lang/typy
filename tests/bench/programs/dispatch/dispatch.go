// Method calls through an interface, as an object-oriented program makes them.
package main

import (
	"fmt"
	"os"
	"strconv"
)

type shape interface{ area() float64 }

type square struct{ side float64 }
type circle struct{ radius float64 }
type rectangle struct{ width, height float64 }

func (s *square) area() float64    { return s.side * s.side }
func (c *circle) area() float64    { return 3.14159 * c.radius * c.radius }
func (r *rectangle) area() float64 { return r.width * r.height }

func main() {
	rounds, _ := strconv.Atoi(os.Args[1])
	made := make([]shape, 0, 1000)
	for i := 0; i < 1000; i++ {
		size := float64(i%7 + 1)
		switch i % 3 {
		case 0:
			made = append(made, &square{size})
		case 1:
			made = append(made, &circle{size})
		default:
			made = append(made, &rectangle{size, size + 1.0})
		}
	}
	found := 0.0
	for r := 0; r < rounds; r++ {
		for _, s := range made {
			found += s.area()
		}
	}
	fmt.Printf("%.3f\n", found)
}
