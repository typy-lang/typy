# typy-test: EXC-14
# typy-expect: rejected E0412 at 4:7
error = OSError(2, "missing")
print(error.errnum)
