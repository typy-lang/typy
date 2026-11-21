# typy-test: CLS-38
# typy-expect: rejected E0201 at 7:2
def register[T](cls: T) -> T:
    return cls


@register
class Box:
    pass


print(Box.__name__)
