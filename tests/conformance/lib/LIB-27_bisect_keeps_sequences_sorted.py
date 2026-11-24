# typy-test: LIB-27
# typy-expect: accepted
# bisect finds and keeps places in sorted sequences as CPython's module does.
import bisect
from bisect import bisect_left, insort_left

grades = [60, 70, 80, 90]
print(bisect.bisect(grades, 75), bisect.bisect_right(grades, 80), bisect_left(grades, 80))
print(bisect.bisect_left(grades, 80, 1, 3), bisect.bisect_right(grades, 10), bisect_left([], 5))
names = ["ant", "cat", "dog"]
bisect.insort(names, "bee")
insort_left(names, "cat")
bisect.insort_right(names, "zebra", 0)
print(names, bisect.bisect(("a", "c", "e"), "d"))
