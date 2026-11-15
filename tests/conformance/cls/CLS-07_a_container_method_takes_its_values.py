# typy-test: CLS-07
# typy-expect: rejected E0404 at 4:9
class Grid:
    def __setitem__(self, key: int) -> None:
        pass
