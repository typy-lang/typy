# typy-test: TYP-02
# typy-expect: accepted
def describe() -> str:
    count = 3
    label = "items"
    count = count + 1
    return str(count) + " " + label


print(describe())
