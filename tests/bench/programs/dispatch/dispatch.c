/* Method calls through a base class, as an object-oriented program makes them: each shape
   holds a pointer to its class's table of functions. */
#include <stdio.h>
#include <stdlib.h>

struct shape;
struct shape_class {
    double (*area)(const struct shape *);
};
struct shape {
    const struct shape_class *class;
    double first, second;
};

static double square_area(const struct shape *s) { return s->first * s->first; }
static double circle_area(const struct shape *s) { return 3.14159 * s->first * s->first; }
static double rectangle_area(const struct shape *s) { return s->first * s->second; }

static const struct shape_class SQUARE = {square_area};
static const struct shape_class CIRCLE = {circle_area};
static const struct shape_class RECTANGLE = {rectangle_area};

int main(int argc, char **argv) {
    long rounds = argc > 1 ? atol(argv[1]) : 100;
    struct shape **made = malloc(1000 * sizeof *made);
    for (int i = 0; i < 1000; i++) {
        double size = (double)(i % 7 + 1);
        struct shape *s = malloc(sizeof *s);
        s->first = size;
        s->second = size + 1.0;
        s->class = i % 3 == 0 ? &SQUARE : i % 3 == 1 ? &CIRCLE : &RECTANGLE;
        made[i] = s;
    }
    double found = 0.0;
    for (long r = 0; r < rounds; r++)
        for (int i = 0; i < 1000; i++) found += made[i]->class->area(made[i]);
    printf("%.3f\n", found);
    for (int i = 0; i < 1000; i++) free(made[i]);
    free(made);
    return 0;
}
