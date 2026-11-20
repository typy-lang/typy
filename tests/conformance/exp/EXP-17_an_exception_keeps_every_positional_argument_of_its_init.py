# typy-test: EXP-17
# typy-expect: accepted
class Failure(Exception):
    def __init__(self, *details: str) -> None:
        super().__init__("failed")
        self.details = details


class Other(Exception):
    def __init__(self, code: int, *details: str, **context: int) -> None:
        self.code = code
        self.details = details
        self.context = context


try:
    raise Failure("a", "b")
except Failure as error:
    print(error.args, error.details, str(error))
try:
    raise Other(3, "x", line=4)
except Other as error:
    print(error.args, error.code, error.details, error.context, str(error))


def both(*args: int, **kwargs: int) -> str:
    return f"{args} {kwargs}"


def values(*items: int) -> tuple[int, ...]:
    return items


print(both(args=1, kwargs=2), both(1, 2))
pair = values(1, 2)
print(pair, pair == (1, 2), values() == (), {values(1): "one"}[(1,)])
