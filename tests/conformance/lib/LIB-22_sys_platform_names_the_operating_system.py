# typy-test: LIB-22
# typy-expect: accepted
import sys
from sys import platform

if sys.platform == "darwin":
    system = "macOS"
elif sys.platform.startswith("linux"):
    system = "Linux"
else:
    system = "another system"
print(system, platform == sys.platform, platform.isalpha(), platform.islower())
