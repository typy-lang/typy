# typy-test: SMT-01
# typy-expect: accepted
xs = [1, 2]
alias = xs
xs += [3]
print(xs, alias)
xs += xs
print(alias)
alias *= 2
print(xs)
xs *= 0
print(alias, len(xs))
