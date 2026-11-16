# typy-test: CLS-06
# typy-expect: accepted
class Temp:
    def __init__(self) -> None:
        self._celsius = 0
        self.history: list[int] = []

    @property
    def celsius(self) -> int:
        print("get")
        return self._celsius

    @celsius.setter
    def celsius(self, value: int) -> None:
        print("set", value)
        self.history.append(value)
        self._celsius = value


class Room:
    def __init__(self) -> None:
        self.temp = Temp()


def room() -> Room:
    print("room")
    return hall


def compute() -> int:
    print("compute")
    return 21


hall = Room()
t = Temp()
t.celsius += 5
hall.temp.celsius = 3
room().temp.celsius = compute()
room().temp.celsius += 2
hall.temp.celsius *= 2
a = b = Temp()
a.celsius = b.celsius = 7
print(t.celsius, hall.temp.celsius, hall.temp.history, a.celsius)
