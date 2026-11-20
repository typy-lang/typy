# typy-test: FUN-11, MOD-01
# typy-expect: accepted
import helper
import other

in_main = helper.g("main global")
next(in_main)
print("main ends")
