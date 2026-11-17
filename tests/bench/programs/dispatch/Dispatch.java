// Method calls through a base class, as an object-oriented program makes them.
import java.util.ArrayList;
import java.util.List;

public final class Dispatch {
    static class Shape {
        double area() {
            return 0.0;
        }
    }

    static final class Square extends Shape {
        final double side;

        Square(double side) {
            this.side = side;
        }

        @Override
        double area() {
            return side * side;
        }
    }

    static final class Circle extends Shape {
        final double radius;

        Circle(double radius) {
            this.radius = radius;
        }

        @Override
        double area() {
            return 3.14159 * radius * radius;
        }
    }

    static final class Rectangle extends Shape {
        final double width, height;

        Rectangle(double width, double height) {
            this.width = width;
            this.height = height;
        }

        @Override
        double area() {
            return width * height;
        }
    }

    public static void main(String[] args) {
        int rounds = Integer.parseInt(args[0]);
        List<Shape> made = new ArrayList<>();
        for (int i = 0; i < 1000; i++) {
            double size = i % 7 + 1;
            if (i % 3 == 0) made.add(new Square(size));
            else if (i % 3 == 1) made.add(new Circle(size));
            else made.add(new Rectangle(size, size + 1.0));
        }
        double found = 0.0;
        for (int r = 0; r < rounds; r++) for (Shape shape : made) found += shape.area();
        System.out.printf("%.3f%n", found);
    }
}
