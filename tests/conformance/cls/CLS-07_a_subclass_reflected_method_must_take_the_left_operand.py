# typy-test: CLS-07
# typy-expect: rejected E0404 at 13:12
class Base:
    pass


class Narrow(Base):
    def __radd__(self, other: "Narrow") -> int:
        return 1


def total(value: Base) -> int:
    return 1 + value


print(total(Narrow()))
