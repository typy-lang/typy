# typy-test: CST-06, TYP-34
# typy-expect: accepted
# A case whose pattern the subject's type never matches never runs, so it is neither
# checked nor compiled, and W0402 says so.
word = "text"
match word:
    case int(number):
        print(number.upper())
    case str():
        print("a str")
