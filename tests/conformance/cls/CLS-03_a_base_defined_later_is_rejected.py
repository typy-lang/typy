# typy-test: CLS-03
# typy-expect: rejected E0306 at 3:13
class Child(Parent):
    pass


class Parent(Exception):
    pass
