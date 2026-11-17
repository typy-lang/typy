# typy-test: CST-06
# typy-expect: rejected E0103 at 5:10
code = 1
match code:
    case (1 as one) | (2 as two):
        print("small")
