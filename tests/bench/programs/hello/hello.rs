//! Startup: a program that prints one line.

fn main() {
    println!("hello {}", std::env::args().count());
}
