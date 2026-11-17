# typy-test: CST-06
# typy-expect: rejected E0103 at 5:10
values = [1, 2, 3]
match values:
    case [*head, *tail]:
        print(head, tail)
