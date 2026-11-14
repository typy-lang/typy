# typy-test: GEN-01
# typy-expect: rejected E0404 at 7:7
def empty[T]() -> list[T]:
    return []


print(empty())
