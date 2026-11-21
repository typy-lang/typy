# typy-test: CLS-40
# typy-expect: rejected E0201 at 10:19
class Shape:
    def __init__(self, size: int) -> None:
        self.size = size


def check(kind: type[Shape], value: Shape) -> int:
    match value:
        case kind(size):
            return size
    return 0
