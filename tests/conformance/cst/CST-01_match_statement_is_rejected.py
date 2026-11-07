# typy-test: CST-01
# typy-expect: rejected E0201 at 4:1
n = 1
match n:
    case 1:
        print("one")
