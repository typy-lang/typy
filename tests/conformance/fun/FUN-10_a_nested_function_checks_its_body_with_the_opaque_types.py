# typy-test: FUN-10
# typy-expect: rejected E0401 at 5:16
def twice[T](value: T) -> T:
    def doubled() -> T:
        return value + value

    return doubled()


print(twice(3))
