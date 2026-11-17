# typy-test: FFI-03
# typy-expect: accepted
import ctypes


class Point(ctypes.Structure):
    _fields_ = [("x", ctypes.c_int), ("y", ctypes.c_double)]


class Box(ctypes.Structure):
    """A box with a corner."""

    _fields_ = [
        ("corner", Point),
        ("size", ctypes.c_int * 2),
        ("tag", ctypes.c_char_p),
        ("next", ctypes.POINTER(Point)),
    ]


class Mixed(ctypes.Structure):
    _fields_ = [
        ("a", ctypes.c_char_p),
        ("b", ctypes.c_int8),
        ("c", ctypes.c_int64),
        ("d", ctypes.c_bool),
    ]


class Empty(ctypes.Structure):
    pass


print(ctypes.sizeof(Point), ctypes.sizeof(Box), ctypes.sizeof(Mixed), ctypes.sizeof(Empty))
p = Point(1, 2.5)
print(p.x, p.y, Point(y=3).x, Point(y=3).y, bool(Point()), p == p)
p.x = 2**33 + 1
p.y += 1
print(p.x, p.y)

box = Box()
print(box.corner.x, list(box.size), box.tag, bool(box.next))
box.corner.x = 7
corner = box.corner
corner.y = 9.5
print(box.corner.x, box.corner.y)
box.size[1] = 4
box.corner = Point(3, 4)
print(list(box.size), box.corner.x, corner.x)
box.tag = bytes([104, 105])
print(box.tag)
box.next = ctypes.pointer(p)
print(box.next.contents.x, box.next[0].y)
box.next = None
print(bool(box.next))

points = (Point * 3)(Point(1, 1.0), Point(2, 2.0))
points[0].x = 10
points[2] = p
print(len(points), [each.x for each in points], points[2].y)
to_point = ctypes.pointer(points[1])
to_point.contents.y = 8.5
print(points[1].y, to_point[0].x)


def area(width: int, height: int) -> float:
    corner = Point(width, height)
    return corner.x * corner.y


print(area(3, 4))
