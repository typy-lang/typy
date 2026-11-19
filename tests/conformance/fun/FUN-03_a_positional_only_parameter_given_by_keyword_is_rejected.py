# typy-test: FUN-03
# typy-expect: rejected E0404 at 7:15
def area(width: int, height: int, /) -> int:
    return width * height


print(area(2, height=3))
