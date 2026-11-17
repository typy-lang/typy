# typy-test: CST-06
# typy-expect: rejected E0407 at 5:10
pair = (1, "a")
match pair:
    case (x, y, z):
        print(x, y, z)
