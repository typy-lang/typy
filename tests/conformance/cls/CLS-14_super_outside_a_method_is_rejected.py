# typy-test: CLS-14
# typy-expect: rejected E0414 at 8:12
class Shape:
    pass


def make() -> None:
    return super().__init__()
