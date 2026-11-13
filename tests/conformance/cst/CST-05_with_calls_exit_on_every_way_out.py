# typy-test: CST-05
# typy-expect: accepted
class Resource:
    def __init__(self, name: str, swallow: bool = False) -> None:
        self.name = name
        self.swallow = swallow

    def __enter__(self) -> "Resource":
        print("enter", self.name)
        return self

    def __exit__(self, kind, error, trace) -> bool:
        if error is None:
            print("exit", self.name, kind, error, trace is None)
        else:
            print("exit", self.name, kind, repr(error), trace is None)
        return self.swallow


class Pair:
    def __enter__(self) -> tuple[int, str]:
        return (1, "one")

    def __exit__(self, kind, error, trace) -> None:
        print("pair done")


def early(n: int) -> int:
    with Resource("early"):
        if n > 0:
            return n * 10
    return -1


with Resource("plain") as resource:
    print("body", resource.name)
print(early(2), early(0))
for i in range(3):
    with Resource(f"loop{i}"):
        if i == 0:
            continue
        if i == 2:
            break
        print("loop body", i)
with Resource("quiet", True):
    raise KeyError("swallowed")
print("after quiet")
with Resource("outer"), Resource("inner", True) as inner:
    print("both", inner.name)
    raise ValueError("inner swallows")
with Pair() as (number, word):
    print(number, word)
try:
    with Resource("loud"):
        raise IndexError("passes")
except IndexError as error:
    print("caught", error)
