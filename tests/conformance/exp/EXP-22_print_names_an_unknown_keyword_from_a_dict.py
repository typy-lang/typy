# typy-test: EXP-22
# typy-expect: accepted
# typy-python: 3.15
options = {"sepx": "-"}
try:
    print("a", **options)
except TypeError as error:
    print(error)
