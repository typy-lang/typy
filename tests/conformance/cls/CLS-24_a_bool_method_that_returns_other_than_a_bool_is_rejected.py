# typy-test: CLS-24
# typy-expect: rejected E0404 at 4:9
class Flag:
    def __bool__(self) -> int:
        return 1


if Flag():
    print("set")
