# typy-test: GEN-07
# typy-expect: rejected E0408 at 5:16
import tools

box: tools.Box[int, str] = tools.Box(1)
print(box.item)
