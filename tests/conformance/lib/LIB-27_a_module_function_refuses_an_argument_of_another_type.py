# typy-test: LIB-27
# typy-expect: rejected E0404 at 5:24
# base64 encodes bytes, so CPython raises TypeError for a str.
import base64
print(base64.b64encode("text"))
