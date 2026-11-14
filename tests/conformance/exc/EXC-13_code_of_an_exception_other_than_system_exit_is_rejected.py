# typy-test: EXC-13
# typy-expect: rejected E0412 at 6:11
try:
    raise ValueError(2)
except ValueError as error:
    print(error.code)
