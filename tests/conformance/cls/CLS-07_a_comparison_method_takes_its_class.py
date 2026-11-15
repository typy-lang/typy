# typy-test: CLS-07
# typy-expect: rejected E0404 at 4:9
class Point:
    def __lt__(self, other: int) -> bool:
        return True
