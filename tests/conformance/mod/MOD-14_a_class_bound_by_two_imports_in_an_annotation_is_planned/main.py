# typy-test: MOD-14
# typy-expect: rejected E0201 at 9:17
import sys
if len(sys.argv) > 5:
    from shapes_a import Shape
else:
    from shapes_b import Shape

def size(shape: Shape) -> int:
    return shape.n
