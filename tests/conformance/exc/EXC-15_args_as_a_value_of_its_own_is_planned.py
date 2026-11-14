# typy-test: EXC-15
# typy-expect: rejected E0201 at 6:11
try:
    raise ValueError("bad", 3)
except ValueError as error:
    print(error.args[0])
