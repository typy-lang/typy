# typy-test: EXC-01, TYP-12
# typy-expect: rejected E0306 at 8:7
error = 1
try:
    raise ValueError("v")
except ValueError as error:
    print(error)
print(error)
