# typy-test: LIB-03
# typy-expect: accepted
xs = [3, 1, 3, 2]
print(xs.index(3), xs.index(2), xs.count(3), xs.count(5))
floats = [0.5, -0.0]
print(floats.index(0.0), floats.count(0.0))
words = ["x", "y"]
print(words.index("y"))
