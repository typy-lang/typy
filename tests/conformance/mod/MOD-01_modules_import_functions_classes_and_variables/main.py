# typy-test: MOD-01, MOD-10, SMT-07
# typy-expect: accepted
import shapes
from shapes import Point, area, ORIGIN
import tools.text
from tools import text as words
from tools.text import shout
import counter
from types import ModuleType


def describe(module: ModuleType) -> str:
    return str(module) + " " + str(module is shapes)


print(shapes)
print(__name__, shapes.NAME, tools.text.__name__ if False else "skip")
p = Point(3, 4)
print(area(p), shapes.area(Point(1, 2)), ORIGIN.x, shapes.ORIGIN is ORIGIN)
print(shout("hi"), words.shout("yo"), tools.text.LOUD)
counter.bump()
counter.bump()
print(counter.count, counter.total())
counter.count = 10
print(counter.total(), shapes is shapes, words is tools.text)
q: shapes.Point = shapes.Point(5, 6)
print(q.x + q.y)
try:
    print(counter.later)
except AttributeError as error:
    print("AttributeError", error)
try:
    from counter import later
except ImportError as error:
    print("ImportError", error)
print(describe(shapes), describe(counter))
