"""Heap queue algorithm (a.k.a. priority queue).

Heaps are arrays for which a[k] <= a[2*k+1] and a[k] <= a[2*k+2] for
all k, counting elements from 0.  For the sake of comparison,
non-existing elements are considered to be infinite.  The interesting
property of a heap is that a[0] is always its smallest element.

Usage:

heap = []            # creates an empty heap
heappush(heap, item) # pushes a new item on the heap
item = heappop(heap) # pops the smallest item from the heap
item = heap[0]       # smallest item on the heap without popping it
heapify(x)           # transforms list into a heap, in-place, in linear time
item = heappushpop(heap, item) # pushes a new item and then returns
                               # the smallest item; the heap size is unchanged
item = heapreplace(heap, item) # pops and returns smallest item, and adds
                               # new item; the heap size is unchanged
"""

from collections.abc import Iterable, Iterator

__all__ = ["heappush", "heappop", "heapify", "heapreplace", "merge", "nlargest", "nsmallest", "heappushpop"]


def heappush[T](heap: list[T], item: T, /) -> None:
    """Push item onto heap, maintaining the heap invariant."""
    heap.append(item)
    _siftdown(heap, 0, len(heap) - 1)


def heappop[T](heap: list[T], /) -> T:
    """Pop the smallest item off the heap, maintaining the heap invariant."""
    if not heap:
        raise IndexError("index out of range")
    lastelt = heap.pop()
    if heap:
        returnitem = heap[0]
        heap[0] = lastelt
        _siftup(heap, 0)
        return returnitem
    return lastelt


def heapreplace[T](heap: list[T], item: T, /) -> T:
    """Pop and return the current smallest value, and add the new item."""
    if not heap:
        raise IndexError("index out of range")
    returnitem = heap[0]
    heap[0] = item
    _siftup(heap, 0)
    return returnitem


def heappushpop[T](heap: list[T], item: T, /) -> T:
    """Fast version of a heappush followed by a heappop."""
    if len(heap) > 0 and heap[0] < item:
        item, heap[0] = heap[0], item
        _siftup(heap, 0)
    return item


def heapify[T](heap: list[T], /) -> None:
    """Transform list into a heap, in-place, in O(len(x)) time."""
    n = len(heap)
    for i in reversed(range(n // 2)):
        _siftup(heap, i)


def _heappop_max[T](heap: list[T]) -> T:
    """Maxheap version of a heappop."""
    lastelt = heap.pop()
    if heap:
        returnitem = heap[0]
        heap[0] = lastelt
        _siftup_max(heap, 0)
        return returnitem
    return lastelt


def _heapreplace_max[T](heap: list[T], item: T) -> T:
    """Maxheap version of a heappop followed by a heappush."""
    returnitem = heap[0]
    heap[0] = item
    _siftup_max(heap, 0)
    return returnitem


def _heapify_max[T](heap: list[T]) -> None:
    """Transform list into a maxheap, in-place, in O(len(x)) time."""
    n = len(heap)
    for i in reversed(range(n // 2)):
        _siftup_max(heap, i)


def _siftdown[T](heap: list[T], startpos: int, pos: int) -> None:
    newitem = heap[pos]
    while pos > startpos:
        parentpos = (pos - 1) >> 1
        parent = heap[parentpos]
        if newitem < parent:
            heap[pos] = parent
            pos = parentpos
            continue
        break
    heap[pos] = newitem


def _siftup[T](heap: list[T], pos: int) -> None:
    endpos = len(heap)
    startpos = pos
    newitem = heap[pos]
    childpos = 2 * pos + 1
    while childpos < endpos:
        rightpos = childpos + 1
        if rightpos < endpos and not heap[childpos] < heap[rightpos]:
            childpos = rightpos
        heap[pos] = heap[childpos]
        pos = childpos
        childpos = 2 * pos + 1
    heap[pos] = newitem
    _siftdown(heap, startpos, pos)


def _siftdown_max[T](heap: list[T], startpos: int, pos: int) -> None:
    newitem = heap[pos]
    while pos > startpos:
        parentpos = (pos - 1) >> 1
        parent = heap[parentpos]
        if parent < newitem:
            heap[pos] = parent
            pos = parentpos
            continue
        break
    heap[pos] = newitem


def _siftup_max[T](heap: list[T], pos: int) -> None:
    endpos = len(heap)
    startpos = pos
    newitem = heap[pos]
    childpos = 2 * pos + 1
    while childpos < endpos:
        rightpos = childpos + 1
        if rightpos < endpos and not heap[rightpos] < heap[childpos]:
            childpos = rightpos
        heap[pos] = heap[childpos]
        pos = childpos
        childpos = 2 * pos + 1
    heap[pos] = newitem
    _siftdown_max(heap, startpos, pos)


def merge[T](*iterables: Iterable[T], reverse: bool = False) -> Iterator[T]:
    """Merge multiple sorted inputs into a single sorted output.

    Similar to sorted(itertools.chain(*iterables)) but returns a generator,
    does not pull the data into memory all at once, and assumes that each of
    the input streams is already sorted (smallest to largest).
    """
    heap: list[tuple[T, int]] = []
    sources: dict[int, Iterator[T]] = {}
    direction = -1 if reverse else 1
    for order, iterable in enumerate(iterables):
        source = iter(iterable)
        for value in source:
            heap.append((value, order * direction))
            sources[order * direction] = source
            break
    if reverse:
        _heapify_max(heap)
    else:
        heapify(heap)
    while len(heap) > 1:
        value, order = heap[0]
        yield value
        advanced = False
        for following in sources[order]:
            if reverse:
                _heapreplace_max(heap, (following, order))
            else:
                heapreplace(heap, (following, order))
            advanced = True
            break
        if not advanced:
            if reverse:
                _heappop_max(heap)
            else:
                heappop(heap)
    if heap:
        value, order = heap[0]
        yield value
        yield from sources[order]


def nsmallest[T](n: int, iterable: Iterable[T]) -> list[T]:
    """Find the n smallest elements in a dataset.

    Equivalent to:  sorted(iterable)[:n]
    """
    source = iter(iterable)
    result = [(elem, i) for i, elem in zip(range(n), source)]
    if not result:
        return []
    _heapify_max(result)
    top = result[0][0]
    order = n
    for elem in source:
        if elem < top:
            _heapreplace_max(result, (elem, order))
            top = result[0][0]
            order += 1
    result.sort()
    return [elem for (elem, order) in result]


def nlargest[T](n: int, iterable: Iterable[T]) -> list[T]:
    """Find the n largest elements in a dataset.

    Equivalent to:  sorted(iterable, reverse=True)[:n]
    """
    source = iter(iterable)
    result = [(elem, i) for i, elem in zip(range(0, -n, -1), source)]
    if not result:
        return []
    heapify(result)
    top = result[0][0]
    order = -n
    for elem in source:
        if top < elem:
            heapreplace(result, (elem, order))
            top = result[0][0]
            order -= 1
    result.sort(reverse=True)
    return [elem for (elem, order) in result]
