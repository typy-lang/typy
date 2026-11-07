# typy-test: STD-06
# typy-expect: rejected E0201 at 7:5
count = 0


def bump() -> None:
    global count
    count = 1
