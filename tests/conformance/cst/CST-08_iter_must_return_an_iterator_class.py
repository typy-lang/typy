# typy-test: CST-08
# typy-expect: rejected E0411 at 8:15
class Numbers:
    def __iter__(self) -> int:
        return 1


for number in Numbers():
    print(number)
