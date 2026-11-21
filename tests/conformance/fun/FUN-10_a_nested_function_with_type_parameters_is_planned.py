# typy-test: FUN-10
# typy-expect: rejected E0201 at 4:9
def outer(value: int) -> int:
    def apply[T](item: T) -> T:
        return item

    return apply(value)


print(outer(3))
