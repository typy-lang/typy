# typy-test: DAT-08
# typy-expect: rejected E0307 at 10:13
def pick(flag: int) -> tuple[str] | tuple[str, int]:
    if flag:
        return ("a", 1)
    return ("b",)


value = pick(1)
print(value[2])
