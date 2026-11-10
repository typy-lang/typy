# Generics

Generic functions and classes, written with PEP 695 type parameter lists,
and protocols come in 0.4 (GEN-01 to GEN-05). TyPy 0.2 accepts no generic
definitions of its own; the built-in generic types `list[T]`, `dict[K, V]`,
`set[T]` and `tuple[...]` are covered in [Types](types.md).

## What TyPy 0.2 accepts

Concrete functions over the built-in generic types:

<!-- typy-expect: accepted -->
```python
def first(items: list[str]) -> str | None:
    if items:
        return items[0]
    return None


print(first(["a", "b"]), first([]))
```

## Planned

A type parameter list is refused until 0.4:

<!-- typy-expect: rejected E0201 at 1:1 -->
```python
def first[T](items: list[T]) -> T:
    return items[0]
```
