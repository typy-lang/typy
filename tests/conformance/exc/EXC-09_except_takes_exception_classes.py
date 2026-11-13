# typy-test: EXC-09
# typy-expect: rejected E0418 at 5:8
try:
    print(1)
except "ValueError":
    print(2)
