# typy-test: CLS-07
# typy-expect: rejected E0401 at 9:1
class Read:
    def __getitem__(self, index: int) -> int:
        return index


r = Read()
r[0] += 1
