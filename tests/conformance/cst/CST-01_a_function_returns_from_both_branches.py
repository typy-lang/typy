# typy-test: CST-01, TYP-01
# typy-expect: accepted
def sign(value: int) -> str:
    if value < 0:
        return "negative"
    else:
        return "not negative"


print(sign(-1), sign(2))
