# typy-test: CLS-23
# typy-expect: accepted
class Window:
    def __init__(self, items: list[int]) -> None:
        self.items = items

    def __getitem__(self, index: int | slice) -> "int | Window":
        if isinstance(index, slice):
            print("slice", index, index.start, index.stop, index.step)
            return Window(self.items[index])
        return self.items[index]

    def __setitem__(self, index: int | slice, value: int) -> None:
        if isinstance(index, slice):
            start, stop, step = index.indices(len(self.items))
            for position in range(start, stop, step):
                self.items[position] = value
        else:
            self.items[index] = value

    def __delitem__(self, index: slice) -> None:
        print("del", index)
        del self.items[index.start:index.stop]

    def __len__(self) -> int:
        return len(self.items)


w = Window([1, 2, 3, 4, 5, 6])
part = w[1:4]
if isinstance(part, Window):
    print(part.items)
whole = w[::2]
if isinstance(whole, Window):
    print(whole.items)
print(w[2])
w[0:2] = 9
w[::-3] = 7
print(w.items)
del w[4:]
print(w.items)
s = slice(1, None, 2)
print(s, repr(s), s.start, s.stop, s.step, slice(3), slice(1, 2) == slice(1, 2, None))
print(s != slice(1, None), s.indices(10), slice(-3, None).indices(5), slice(None, None, -1).indices(4))
print("abcdef"[s], [10, 20, 30, 40][slice(None, None, -1)], b"bytes"[slice(1, 3)], range(10)[s])
print(len({slice(1, 2), slice(1, 2), slice(2)}), type(s).__name__, isinstance(s, slice))
for bad in [-1, 2]:
    try:
        print(slice(0, 1, 0 if bad > 0 else 1).indices(bad))
    except ValueError as error:
        print("ValueError:", error)
