# typy-test: EXC-08
# typy-expect: rejected E0404 at 4:7
error = ValueError("x")
print(repr(error, 1))
