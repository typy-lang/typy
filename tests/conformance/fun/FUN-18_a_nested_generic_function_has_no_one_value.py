# typy-test: FUN-18
# typy-expect: rejected E0202 at 7:12
def outer() -> None:
    def pair[T](first: T, second: T) -> tuple[T, T]:
        return (first, second)

    held = pair
    print(held)


outer()
