# typy-test: EXC-10
# typy-expect: accepted
missing = AttributeError("no such attribute")
later: RuntimeError = NotImplementedError("later")
print(repr(missing), repr(later))
raise later
