# typy-test: GEN-16
# typy-expect: rejected E0201 at 9:11
class R:
    @classmethod
    def make[U](cls, extra: U) -> U:
        return extra


print(R().make(1))
