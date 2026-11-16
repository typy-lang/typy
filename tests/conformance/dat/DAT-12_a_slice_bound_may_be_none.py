# typy-test: DAT-12
# typy-expect: accepted
def cut(items: list[int], start: int | None, stop: int | None, step: int | None) -> list[int]:
    return items[start:stop:step]


values = [1, 2, 3, 4, 5]
print(cut(values, None, 3, None), cut(values, 1, None, 2), cut(values, None, None, -1))
print("hello"[None:2], b"bytes"[1:None], (1, 2, 3)[None:2], range(10)[None:5:None])
values[None:2] = [9]
del values[3:None]
print(values)
low: int | None = None
print("abc"[low:], [1, 2][low:low:low])
