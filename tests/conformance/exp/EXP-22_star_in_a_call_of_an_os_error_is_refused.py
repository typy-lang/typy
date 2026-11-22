# typy-test: EXP-22
# typy-expect: rejected E0202 at 4:7
details = ["gone"]
raise FileNotFoundError(*details)
