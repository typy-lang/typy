# typy-test: FUN-04
# typy-expect: accepted
def outer(nothing: None) -> bool:
    def inner() -> bool:
        return nothing is None

    return inner()


print(outer(None))


def marked() -> bool:
    marker: None = None

    def inner() -> bool:
        return marker is None

    marker = None
    return inner()


print(marked())
