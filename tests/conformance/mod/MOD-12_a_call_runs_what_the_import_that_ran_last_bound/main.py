# typy-test: MOD-12
# typy-expect: accepted
# typy-python: 3.15
import sys
lazy from first import run
print(run())
lazy from second import run
print(run())
if len(sys.argv) > 5:
    from shapes_a import Shape
else:
    from shapes_b import Shape
shape = Shape(2)
print(shape.n)
if len(sys.argv) > 5:
    from first import late
try:
    print(late())
except NameError as error:
    print("NameError:", error)


def again() -> int:
    from first import run
    return run()


print(again(), run())
