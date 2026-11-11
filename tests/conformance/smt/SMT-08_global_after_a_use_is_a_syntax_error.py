# typy-test: SMT-08, STD-01
# typy-expect: rejected E0103 at 8:12
count = 0


def bump() -> None:
    print(count)
    global count
    count = 1
