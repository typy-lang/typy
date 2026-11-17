// Method calls through a base class, as an object-oriented program makes them.
#include <cstdio>
#include <cstdlib>
#include <memory>
#include <vector>

struct Shape {
    virtual ~Shape() = default;
    virtual double area() const { return 0.0; }
};
struct Square : Shape {
    double side;
    explicit Square(double side) : side(side) {}
    double area() const override { return side * side; }
};
struct Circle : Shape {
    double radius;
    explicit Circle(double radius) : radius(radius) {}
    double area() const override { return 3.14159 * radius * radius; }
};
struct Rectangle : Shape {
    double width, height;
    Rectangle(double width, double height) : width(width), height(height) {}
    double area() const override { return width * height; }
};

int main(int argc, char **argv) {
    long rounds = argc > 1 ? std::atol(argv[1]) : 100;
    std::vector<std::unique_ptr<Shape>> made;
    for (int i = 0; i < 1000; i++) {
        double size = i % 7 + 1;
        if (i % 3 == 0)
            made.push_back(std::make_unique<Square>(size));
        else if (i % 3 == 1)
            made.push_back(std::make_unique<Circle>(size));
        else
            made.push_back(std::make_unique<Rectangle>(size, size + 1.0));
    }
    double found = 0.0;
    for (long r = 0; r < rounds; r++)
        for (const auto &shape : made) found += shape->area();
    std::printf("%.3f\n", found);
}
