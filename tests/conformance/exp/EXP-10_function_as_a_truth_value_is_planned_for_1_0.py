# typy-test: EXP-10
# typy-expect: rejected E0201 at 7:4
def check() -> bool:
    return True


if check:
    print("x")
