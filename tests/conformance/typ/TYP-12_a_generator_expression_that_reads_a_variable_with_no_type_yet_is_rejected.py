# typy-test: TYP-12
# typy-expect: rejected E0306 at 6:14
# A generator expression's body is checked where it stands, so a variable it reads needs a
# type there, as a lambda's does.
def late() -> None:
    scaled = (value * factor for value in range(3))
    factor = 10
    print(list(scaled))


late()
