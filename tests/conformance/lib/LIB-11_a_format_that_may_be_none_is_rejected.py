# typy-test: LIB-11
# typy-expect: rejected E0401 at 4:7
format: str | None = None
print(format % 1)
