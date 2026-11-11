# typy-test: STD-06
# typy-expect: rejected E0306 at 3:1
greet()


def greet() -> None:
    print("hi")
