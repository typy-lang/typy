# typy-test: GEN-03
# typy-expect: rejected E0428 at 6:12
def depth[T](value: T, levels: int) -> int:
    if levels == 0:
        return 0
    return depth([value], levels - 1) + 1


print(depth(1, 3))
