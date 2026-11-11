import shapes

count = 0
ready = False
if ready:
    later = 1


def bump() -> None:
    global count
    count += 1


def total() -> int:
    return count + shapes.area(shapes.ORIGIN)
