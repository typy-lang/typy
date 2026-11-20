# typy-test: TYP-14
# typy-expect: accepted
def show(values: tuple[int, ...] | str) -> str:
    if isinstance(values, str):
        return "text " + values
    return "numbers " + str(values) + " " + str(len(values))


pair = (3, 4)
kept: tuple[int, ...] | str = (5,)
print(show((1, 2)), show(()), show(pair), show("a"), show(kept))
