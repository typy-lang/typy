# typy-test: TYP-18
# typy-expect: accepted
import shapes
from shapes import Origin, Pair


class Point:
    pass


print(Pair.__value__, Origin.__value__, Pair.__type_params__)
print(Origin.__value__ is shapes.Point, Origin.__value__ is Point, shapes.Pair.__value__)
