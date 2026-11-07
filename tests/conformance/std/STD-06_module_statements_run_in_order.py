# typy-test: STD-06
# typy-expect: accepted
print("first")
print(__name__)


def later() -> str:
    return "third"


print("second")
print(later())
