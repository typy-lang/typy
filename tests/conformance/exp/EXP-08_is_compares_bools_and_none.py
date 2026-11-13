# typy-test: EXP-08, TYP-06
# typy-expect: accepted
def answer(code: int) -> bool | None:
    if code == 0:
        return None
    return code > 1


values: list[bool | None] = [None, True, False]
for left in values:
    for right in values:
        print(left, right, left is right, left is not right)
    print(left is True, left is False, left is None, True is left, False is not left)
known: bool = True
for code in range(3):
    flag = answer(code)
    print(code, flag is True, flag is known, known is flag, flag is not False)
    if flag is None:
        print("unknown")
