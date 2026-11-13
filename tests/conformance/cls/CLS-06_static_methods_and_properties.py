# typy-test: CLS-06
# typy-expect: accepted
class Temperature:
    def __init__(self, celsius: float) -> None:
        self._celsius = celsius

    @staticmethod
    def from_fahrenheit_value(fahrenheit: float) -> float:
        return (fahrenheit - 32.0) * 5.0 / 9.0

    @property
    def celsius(self) -> float:
        return self._celsius

    @celsius.setter
    def celsius(self, value: float) -> None:
        if value < -273.15:
            raise ValueError("below absolute zero")
        self._celsius = value

    @property
    def fahrenheit(self) -> float:
        return self._celsius * 9.0 / 5.0 + 32.0

    def describe(self) -> str:
        return f"{self.celsius:.1f}C is {self.fahrenheit:.1f}F"


class Kelvin(Temperature):
    @property
    def celsius(self) -> float:
        return self._celsius + 0.0

    @property
    def fahrenheit(self) -> float:
        return 0.0


t = Temperature(Temperature.from_fahrenheit_value(212.0))
print(t.describe(), t.celsius, t.from_fahrenheit_value(32.0))
t.celsius = 25.0
print(t.describe())
try:
    t.celsius = -500.0
except ValueError as error:
    print("error:", error)
temps: list[Temperature] = [t, Kelvin(10.0)]
for each in temps:
    print(each.describe(), each.fahrenheit)
