# typy-test: LEX-07
# typy-expect: accepted
for value in [2**63, -2**63 - 1, 2**63 - 1, -2**63, -1, 65]:
    try:
        print(f"{value:c}")
    except OverflowError as error:
        print(error)
