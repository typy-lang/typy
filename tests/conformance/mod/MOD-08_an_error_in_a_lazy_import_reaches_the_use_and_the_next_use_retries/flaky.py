import tries

tries.count += 1
if tries.count < 3:
    raise ValueError("not yet")
READY = "ready"
