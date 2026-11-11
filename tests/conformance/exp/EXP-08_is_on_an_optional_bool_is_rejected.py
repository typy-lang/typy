# typy-test: EXP-08
# typy-expect: rejected E0301 at 4:15
flag: bool | None = None
print(flag is True)
