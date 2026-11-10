# typy-test: CST-03
# typy-expect: accepted
for letter in "na\u00efve \U0001f600":
    print(letter, ord(letter))
else:
    print("done")
for letter in "":
    print("never")
