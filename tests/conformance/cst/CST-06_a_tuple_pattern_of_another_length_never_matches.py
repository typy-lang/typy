# typy-test: CST-06, TYP-34
# typy-expect: accepted
# A tuple of two items never matches a pattern of three, so the case never runs.
pair = (1, "a")
match pair:
    case (x, y, z):
        print(x, y, z)
    case (x, y):
        print(y, x)
