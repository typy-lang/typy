# typy-test: EXP-22
# typy-expect: rejected E0201 at 4:19
separators = [","]
print("a,b".split(*separators))
