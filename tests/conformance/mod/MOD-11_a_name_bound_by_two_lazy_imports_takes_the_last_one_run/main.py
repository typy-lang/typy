# typy-test: MOD-11
# typy-expect: accepted
# typy-python: 3.15
lazy from first import value

print("start")
print(value)
lazy from second import value


def show() -> None:
    print(value)


print("before show")
show()
lazy from first import value
lazy from second import value
print("again")
print(value)
