# typy-test: FUN-01
# typy-expect: accepted
def area(width: int, height: int) -> int:
    return width * height


def report() -> None:
    print(area(3, 4))


report()
