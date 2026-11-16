# typy-test: CST-11
# typy-expect: rejected E0201 at 10:7
class Letters:
    def __getitem__(self, index: int) -> str:
        if index >= 3:
            raise IndexError(index)
        return "abc"[index]


print("b" in Letters())
