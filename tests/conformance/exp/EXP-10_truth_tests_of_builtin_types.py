# typy-test: EXP-10
# typy-expect: accepted
for value in range(3):
    if value:
        print("int", value)
if "":
    print("never")
if 0.5 and 2.5:
    print("floats true")
if "x":
    print("str true")
if not 0.0:
    print("zero float false")
