# typy-test: SMT-12
# typy-expect: accepted
import shapes
from shapes import Number

type Score = int
type Grid[T] = list[list[T]]

print(Score, repr(Score), str(Score), f"[{Score}]", Score.__name__, Score.__module__)
print(Grid, Number, shapes.Pair, shapes.Pair.__module__, Number.__name__)
print(type(Score), type(Score).__name__, Score is Score, Number is shapes.Number)
kinds = [Score, Grid, Number]
for kind in kinds:
    print(kind, kind.__name__, kind.__module__)
saved = shapes.Pair
print(saved is shapes.Pair, saved is Score)
