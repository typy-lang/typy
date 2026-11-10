# typy-test: CST-03
# typy-expect: rejected E0411 at 4:10
pair = (1, "a")
for x in pair:
    print(x)
