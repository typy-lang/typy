# typy-test: MOD-06
# typy-expect: rejected E0103 at 5:5
# typy-python: 3.15
try:
    lazy import json
except ImportError:
    pass
