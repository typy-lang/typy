# typy-test: EXP-17
# typy-expect: rejected E0404 at 7:19
def joined(*words: str) -> str:
    return " ".join(words)


print(joined("a", 1))
