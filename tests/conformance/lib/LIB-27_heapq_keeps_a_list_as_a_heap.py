# typy-test: LIB-27
# typy-expect: accepted
# heapq keeps a list as a heap, merges sorted inputs and picks the smallest and largest
# items as CPython's module does, ties broken alike.
import heapq
from heapq import heappop, heappush

heap: list[int] = []
for value in [5, 1, 4, 1, 5, 9, 2, 6]:
    heappush(heap, value)
print(heap, [heappop(heap) for _ in range(3)], heap)
numbers = [5, 1, 4, 1, 5, 9, 2, 6]
heapq.heapify(numbers)
print(numbers, heapq.heappushpop(numbers, 0), heapq.heapreplace(numbers, 7), numbers)
tasks = [(2, "write"), (1, "plan"), (3, "test"), (1, "agree")]
heapq.heapify(tasks)
print([heapq.heappop(tasks) for _ in range(len(tasks))])
print(heapq.nlargest(3, [5, 1, 4, 1, 5, 9, 2, 6]), heapq.nsmallest(2, [(1, "b"), (1, "a"), (0, "z")]))
print(list(heapq.merge([1, 4, 7], [2, 5], [3, 6, 8])), list(heapq.merge([7, 4, 1], [5, 2], reverse=True)))
print(heapq.nsmallest(-1, [3]), heapq.nlargest(10, (3, 1, 2)))
empty: list[int] = []
try:
    heapq.heappop(empty)
except IndexError as error:
    print(error)
