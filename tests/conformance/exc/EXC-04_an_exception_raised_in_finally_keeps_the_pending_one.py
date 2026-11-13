# typy-test: EXC-04, CST-04, STD-05
# typy-expect: accepted
try:
    raise KeyError("pending")
finally:
    raise RuntimeError("in finally")
