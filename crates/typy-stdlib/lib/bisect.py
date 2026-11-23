"""Bisection algorithms."""

from collections.abc import MutableSequence, Sequence


def insort_right[T](a: MutableSequence[T], x: T, lo: int = 0, hi: int | None = None) -> None:
    """Insert item x in list a, and keep it sorted assuming a is sorted.

    If x is already in a, insert it to the right of the rightmost x.

    Optional args lo (default 0) and hi (default len(a)) bound the
    slice of a to be searched.
    """
    lo = bisect_right(a, x, lo, hi)
    a.insert(lo, x)


def bisect_right[T](a: Sequence[T], x: T, lo: int = 0, hi: int | None = None) -> int:
    """Return the index where to insert item x in list a, assuming a is sorted.

    The return value i is such that all e in a[:i] have e <= x, and all e in
    a[i:] have e > x.  So if x already appears in the list, a.insert(i, x) will
    insert just after the rightmost x already there.

    Optional args lo (default 0) and hi (default len(a)) bound the
    slice of a to be searched.
    """
    # CPython 3.12 to 3.14 give this text; 3.15 says "lo cannot be negative".
    if lo < 0:
        raise ValueError("lo must be non-negative")
    end = len(a) if hi is None else hi
    while lo < end:
        mid = (lo + end) // 2
        if x < a[mid]:
            end = mid
        else:
            lo = mid + 1
    return lo


def insort_left[T](a: MutableSequence[T], x: T, lo: int = 0, hi: int | None = None) -> None:
    """Insert item x in list a, and keep it sorted assuming a is sorted.

    If x is already in a, insert it to the left of the leftmost x.

    Optional args lo (default 0) and hi (default len(a)) bound the
    slice of a to be searched.
    """
    lo = bisect_left(a, x, lo, hi)
    a.insert(lo, x)


def bisect_left[T](a: Sequence[T], x: T, lo: int = 0, hi: int | None = None) -> int:
    """Return the index where to insert item x in list a, assuming a is sorted.

    The return value i is such that all e in a[:i] have e < x, and all e in
    a[i:] have e >= x.  So if x already appears in the list, a.insert(i, x) will
    insert just before the leftmost x already there.

    Optional args lo (default 0) and hi (default len(a)) bound the
    slice of a to be searched.
    """
    if lo < 0:
        raise ValueError("lo must be non-negative")
    end = len(a) if hi is None else hi
    while lo < end:
        mid = (lo + end) // 2
        if a[mid] < x:
            lo = mid + 1
        else:
            end = mid
    return lo


def bisect[T](a: Sequence[T], x: T, lo: int = 0, hi: int | None = None) -> int:
    """Return the index where to insert item x in list a, as bisect_right does."""
    return bisect_right(a, x, lo, hi)


def insort[T](a: MutableSequence[T], x: T, lo: int = 0, hi: int | None = None) -> None:
    """Insert item x in list a, and keep it sorted, as insort_right does."""
    insort_right(a, x, lo, hi)
