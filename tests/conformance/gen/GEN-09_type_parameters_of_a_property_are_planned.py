# typy-test: GEN-09
# typy-expect: rejected E0201 at 5:9
class Size:
    @property
    def value[T](self) -> int:
        return 1
