# typy-test: CST-04
# typy-expect: rejected E0201 at 6:5
try:
    print(1)
finally:
    print(2)
