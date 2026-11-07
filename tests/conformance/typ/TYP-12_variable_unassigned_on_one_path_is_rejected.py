# typy-test: TYP-12
# typy-expect: rejected E0306 at 6:7
flag = True
if flag:
    result = 1
print(result)
