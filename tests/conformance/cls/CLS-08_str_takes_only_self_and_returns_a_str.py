# typy-test: CLS-08
# typy-expect: rejected E0404 at 4:9
class Price:
    def __str__(self) -> int:
        return 1
