# typy-test: GEN-16
# typy-expect: rejected E0201 at 4:9
class R:
    def __eq__[U](self, other: U) -> bool:
        return True
