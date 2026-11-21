# typy-test: EXP-11
# typy-expect: accepted
def pick(flag: bool) -> None:
    return None if flag else None


print(pick(True), pick(False))
