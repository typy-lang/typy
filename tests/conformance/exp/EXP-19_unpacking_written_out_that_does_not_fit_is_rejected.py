# typy-test: EXP-19
# typy-expect: rejected E0404 at 4:7
point = (1, 2)
print(len(*point))
