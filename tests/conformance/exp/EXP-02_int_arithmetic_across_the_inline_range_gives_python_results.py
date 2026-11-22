# typy-test: EXP-02, DAT-01
# typy-expect: accepted
# Each operation runs on the words in place while both values and the result fit in 63
# bits, and in the runtime past them; the results are the same either way.
EDGE = 2**62
VALUES = [0, 1, -1, 7, -7, EDGE - 1, EDGE, -EDGE, -EDGE - 1, 3037000499, -3037000500, 2**64 + 1]

for a in VALUES:
    print(a, -a, ~a)
    for b in VALUES:
        print(a + b, a - b, a * b, a & b, a | b, a ^ b, a < b, a <= b, a == b, a != b)

count = EDGE - 4
for _ in range(8):
    count += 1
print(count, count - EDGE, -count)
