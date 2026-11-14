# typy-test: GEN-02
# typy-expect: rejected E0404 at 7:7
def doubled[N: (int, float)](value: N) -> N:
    return value + value


print(doubled("a"))
