"""The built-in frozendict of Python 3.15 (PEP 814), written in typed Python."""

# A module at the Python floor 3.15 that names `frozendict`, and binds no such name itself,
# imports it from here (DAT-18, ADR-0123). The class shows as the built-in `frozendict`.

from collections.abc import ItemsView, Iterator, KeysView, ValuesView

# The hash of a frozendict is CPython's hash of the frozenset of its items.
_MASK = 0xFFFFFFFFFFFFFFFF


class frozendict[K, V]:
    """An immutable mapping, hashable when its values are."""

    def __init__(self, items: dict[K, V]) -> None:
        self._items = dict(items)

    def __getitem__(self, key: K) -> V:
        return self._items[key]

    def __len__(self) -> int:
        return len(self._items)

    def __iter__(self) -> Iterator[K]:
        return iter(self._items)

    def __contains__(self, key: K) -> bool:
        return key in self._items

    def get(self, key: K, default: V | None = None) -> V | None:
        """Return the value for key if key is in the dictionary, else default."""
        if key in self._items:
            return self._items[key]
        return default

    def keys(self) -> KeysView[K]:
        return self._items.keys()

    def values(self) -> ValuesView[V]:
        return self._items.values()

    def items(self) -> ItemsView[K, V]:
        return self._items.items()

    def copy(self) -> "frozendict[K, V]":
        """Return the frozendict itself, which never changes."""
        return self

    def __or__(self, other: "frozendict[K, V] | dict[K, V]") -> "frozendict[K, V]":
        merged = dict(self._items)
        if isinstance(other, frozendict):
            merged.update(other._items)
        else:
            merged.update(other)
        return frozendict(merged)

    def __eq__(self, other: "frozendict[K, V] | dict[K, V]") -> bool:
        if isinstance(other, frozendict):
            return self._items == other._items
        return self._items == other

    def __hash__(self) -> int:
        total = 0
        for key, value in self._items.items():
            item = hash((key, value)) & _MASK
            total ^= (((item ^ 89869747) ^ ((item << 16) & _MASK)) * 3644798167) & _MASK
        total ^= ((len(self._items) + 1) * 1927868237) & _MASK
        total ^= (total >> 11) ^ (total >> 25)
        total = (total * 69069 + 907133923) & _MASK
        if total == _MASK:
            total = 590923713
        if total >= 1 << 63:
            total -= 1 << 64
        return total

    def __repr__(self) -> str:
        if len(self._items) == 0:
            return "frozendict()"
        return "frozendict(" + repr(self._items) + ")"
