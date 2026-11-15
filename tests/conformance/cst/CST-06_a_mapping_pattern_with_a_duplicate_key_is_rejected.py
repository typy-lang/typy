# typy-test: CST-06
# typy-expect: rejected E0103 at 5:10
settings = {"width": 1}
match settings:
    case {1: a, True: b}:
        print(a, b)
