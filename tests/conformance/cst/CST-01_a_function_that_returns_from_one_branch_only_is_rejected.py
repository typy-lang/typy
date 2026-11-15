# typy-test: CST-01
# typy-expect: rejected E0405 at 3:1
def sign(value: int) -> int:
    if value < 0:
        return -1
    elif value > 0:
        return 1
