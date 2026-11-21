# typy-test: EXP-12, DAT-17
# typy-expect: accepted
try:
    print([index for index in range(0, 5, 0)])
except ValueError as error:
    print(error)
