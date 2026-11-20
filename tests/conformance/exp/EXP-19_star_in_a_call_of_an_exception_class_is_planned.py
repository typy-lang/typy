# typy-test: EXP-19
# typy-expect: rejected E0201 at 8:15
class Failure(Exception):
    pass


values = ["a"]
raise Failure(*values)
