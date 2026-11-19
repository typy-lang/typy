# typy-test: FUN-05
# typy-expect: accepted
class Quiet:
    def name(self) -> str:
        return "quiet"


class Loud:
    def __init__(self, fail: bool) -> None:
        self.fail = fail

    def name(self) -> str:
        return "loud"

    def __repr__(self) -> str:
        if self.fail:
            raise ValueError("no text")
        return "Loud()"


print(repr(Quiet().name).startswith("<bound method Quiet.name of <__main__.Quiet object at 0x"))
print(repr(Loud(False).name), str(Loud(False).name))
try:
    print(repr(Loud(True).name))
except ValueError as error:
    print("ValueError:", error)
quiet = Quiet()
methods = {quiet.name: 1}
print(methods[quiet.name], quiet.name != Quiet().name, quiet.name == quiet.name)
