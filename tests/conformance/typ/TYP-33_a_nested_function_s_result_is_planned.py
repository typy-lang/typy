# typy-test: TYP-33
# typy-expect: rejected E0204 at 4:9
def outer() -> int:
    def inner():
        return 1

    return inner()


print(outer())
