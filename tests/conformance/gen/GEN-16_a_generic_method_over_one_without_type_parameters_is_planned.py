# typy-test: GEN-16
# typy-expect: rejected E0201 at 9:9
class Base:
    def get(self, value: int) -> int:
        return value


class Derived(Base):
    def get[T](self, value: T) -> T:
        return value
