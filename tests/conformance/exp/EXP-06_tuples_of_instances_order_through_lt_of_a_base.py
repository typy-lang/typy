# typy-test: EXP-06
# typy-expect: accepted
# typy-python: 3.14
class Base:
    def __lt__(self, other: Base) -> bool:
        return True


class Derived(Base):
    pass


print((Base(), 1) < (Derived(), 2))
