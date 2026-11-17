//! Method calls through a trait object, as an object-oriented program makes them.

trait Shape {
    fn area(&self) -> f64;
}

struct Square {
    side: f64,
}
struct Circle {
    radius: f64,
}
struct Rectangle {
    width: f64,
    height: f64,
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.side * self.side
    }
}
impl Shape for Circle {
    fn area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }
}
impl Shape for Rectangle {
    fn area(&self) -> f64 {
        self.width * self.height
    }
}

fn main() {
    let rounds: i64 = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(100);
    let made: Vec<Box<dyn Shape>> = (0..1000)
        .map(|i| {
            let size = (i % 7 + 1) as f64;
            match i % 3 {
                0 => Box::new(Square { side: size }) as Box<dyn Shape>,
                1 => Box::new(Circle { radius: size }),
                _ => Box::new(Rectangle { width: size, height: size + 1.0 }),
            }
        })
        .collect();
    let mut found = 0.0;
    for _ in 0..rounds {
        for shape in &made {
            found += shape.area();
        }
    }
    println!("{found:.3}");
}
