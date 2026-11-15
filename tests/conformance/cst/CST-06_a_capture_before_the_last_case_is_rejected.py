# typy-test: CST-06
# typy-expect: rejected E0103 at 5:10
code = 1
match code:
    case other:
        print(other)
    case 1:
        print("one")
