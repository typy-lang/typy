# typy-test: TYP-17
# typy-expect: rejected E0401 at 4:21
import typing
print(isinstance(1, typing.Optional))
