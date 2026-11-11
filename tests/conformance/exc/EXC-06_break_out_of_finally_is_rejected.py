# typy-test: EXC-06
# typy-expect: rejected E0419 at 7:9
for i in range(3):
    try:
        print(i)
    finally:
        break
