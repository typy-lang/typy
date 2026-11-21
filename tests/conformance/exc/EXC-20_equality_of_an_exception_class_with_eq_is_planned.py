# typy-test: EXC-20
# typy-expect: rejected E0201 at 8:23
class Problem(Exception):
    def __eq__(self, other: "Problem") -> bool:
        return str(self) == str(other)


print(Problem("a") == Problem("a"))
