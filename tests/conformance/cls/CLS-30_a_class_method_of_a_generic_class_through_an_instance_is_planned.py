# typy-test: CLS-30
# typy-expect: rejected E0201 at 12:14
class Box[T]:
    def __init__(self, value: T) -> None:
        self.value = value

    @classmethod
    def label(cls) -> str:
        return cls.__name__


print(Box(1).label())
