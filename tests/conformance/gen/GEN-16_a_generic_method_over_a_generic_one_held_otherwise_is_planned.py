# typy-test: GEN-16
# typy-expect: rejected E0201 at 9:9
class Base:
    def get[T](self, value: T) -> int | None:
        return None


class Derived(Base):
    def get[T](self, value: T) -> int:
        return 1
