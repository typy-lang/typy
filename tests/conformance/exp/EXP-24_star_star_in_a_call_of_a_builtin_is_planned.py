# typy-test: EXP-24
# typy-expect: rejected E0201 at 4:22
options = {"reverse": True}
print(sorted([2, 1], **options))
