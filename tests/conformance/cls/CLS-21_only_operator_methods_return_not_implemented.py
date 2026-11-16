# typy-test: CLS-21
# typy-expect: rejected E0405 at 5:16
class Box:
    def size(self) -> int:
        return NotImplemented


print(Box().size())
