# typy-test: FUN-05
# typy-expect: rejected E0404 at 8:25
def twice(value: int) -> int:
    return value * 2


words = ["pear", "fig"]
print(sorted(words, key=twice))
