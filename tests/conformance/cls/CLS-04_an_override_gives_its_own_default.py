# typy-test: CLS-04
# typy-expect: accepted
class Greeter:
    def greet(self, name: str = "world", times: int = 1) -> str:
        return ("hello " + name + " ") * times


class Loud(Greeter):
    def greet(self, name: str = "WORLD", times: int = 1) -> str:
        return ("HELLO " + name + " ") * times


class Louder(Loud):
    def greet(self, name: str = "EVERYONE", times: int = 2) -> str:
        return ("HEY " + name + " ") * times


class Quiet(Loud):
    pass


def run(greeter: Greeter) -> str:
    return greeter.greet()


for each in [Greeter(), Loud(), Louder(), Quiet()]:
    print(run(each), "|", each.greet(times=1), "|", each.greet("ann"))
