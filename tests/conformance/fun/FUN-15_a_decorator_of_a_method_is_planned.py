# typy-test: FUN-15
# typy-expect: rejected E0201 at 8:6
def trace[T](function: T) -> T:
    return function


class Box:
    @trace
    def size(self) -> int:
        return 3


print(Box().size())
