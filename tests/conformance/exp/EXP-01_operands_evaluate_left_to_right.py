# typy-test: EXP-01
# typy-expect: accepted
def say(text: str, value: int) -> int:
    print(text)
    return value


print(say("left", 1) + say("right", 2) * say("last", 3))
