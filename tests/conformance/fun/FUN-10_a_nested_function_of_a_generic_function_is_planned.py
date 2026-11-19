# typy-test: FUN-10
# typy-expect: rejected E0201 at 4:9
def first[T](items: list[T]) -> T:
    def pick() -> T:
        return items[0]
    return pick()
