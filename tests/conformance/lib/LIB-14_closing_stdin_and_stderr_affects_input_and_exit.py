# typy-test: LIB-14
# typy-expect: accepted
import sys

sys.stdin.close()
try:
    input()
except ValueError as error:
    print("input", error)
sys.stderr.close()
sys.exit("bye")
