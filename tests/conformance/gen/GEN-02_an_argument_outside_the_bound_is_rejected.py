# typy-test: GEN-02
# typy-expect: rejected E0404 at 11:7
class Shape:
    pass


def keep[S: Shape](shape: S) -> S:
    return shape


print(keep(1))
