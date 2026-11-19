# typy-test: FUN-05, EXP-02
# typy-expect: rejected E0401 at 13:11
class Box:
    def __init__(self, size: int) -> None:
        self.size_value = size

    @property
    def size(self) -> int:
        return self.size_value


box = Box(3)
print(box.size())
