# typy-test: EXC-09
# typy-expect: accepted
for divisor in [0, 1]:
    try:
        print(10 // divisor)
        raise KeyError("k")
    except ZeroDivisionError:
        print("division")
    except (KeyError, IndexError) as error:
        print("lookup", repr(error))
try:
    raise RuntimeError("r")
except:
    print("bare")
