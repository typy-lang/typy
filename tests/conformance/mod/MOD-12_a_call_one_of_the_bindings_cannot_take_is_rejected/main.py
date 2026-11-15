# typy-test: MOD-12
# typy-expect: rejected E0404 at 9:11
import sys
if len(sys.argv) > 5:
    from first import run
else:
    from second import run

print(run("a"))
