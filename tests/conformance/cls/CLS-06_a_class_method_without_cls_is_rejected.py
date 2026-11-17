# typy-test: CLS-06
# typy-expect: rejected E0404 at 5:9
class Shape:
    @classmethod
    def unit() -> int:
        return 1
