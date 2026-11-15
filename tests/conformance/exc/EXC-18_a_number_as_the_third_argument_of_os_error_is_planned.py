# typy-test: EXC-18
# typy-expect: rejected E0201 at 3:29
error = OSError(11, "busy", 5)
print(error)
