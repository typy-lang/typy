# typy-test: SMT-10
# typy-expect: rejected E0201 at 3:6
type Tree = list[Tree]
