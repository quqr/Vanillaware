import re
import sys

sys.path.insert(0, "F:/Codes/Vanillaware/tmp/rev")
from nso import Image, BASE as VADDR
from vc import load, body

img = Image()
index = load()

RB = re.compile(r"c->x\[(\d+)\] = \(uint64_t\)\((0x7000[0-9a-f]{3}000)ull\);")
RA = re.compile(r"c->x\[(\d+)\] = \(uint64_t\)\(c->x\[(\d+)\] \+ (0x[0-9a-f]+)ull\);")
RG = re.compile(r"c->x\[(\d+)\] = \(uint64_t\)\((0x7002[0-9a-f]{3}000)ull\);")
RL = re.compile(r"\{ uint64_t ea = c->x\[(\d+)\] \+ (0x[0-9a-f]+)ull; uint64_t v = LD64\(ea\);")


def pairs(func):
    lines = body(index, func).split("\n")
    bases, gbases, found = {}, {}, []
    pending = None
    for line in lines:
        m = RB.search(line)
        if m:
            bases[m.group(1)] = int(m.group(2), 16)
            continue
        m = RA.search(line)
        if m and m.group(1) == m.group(2) and m.group(1) in bases:
            addr = bases.pop(m.group(1)) + int(m.group(3), 16)
            if VADDR + img.rodata[0] <= addr < VADDR + img.rodata[0] + img.rodata[1]:
                pending = addr
            continue
        m = RG.search(line)
        if m:
            gbases[m.group(1)] = int(m.group(2), 16)
            continue
        m = RL.search(line)
        if m and m.group(1) in gbases:
            slot = gbases.pop(m.group(1)) + int(m.group(2), 16)
            try:
                value = img.u64(slot)
            except Exception:
                continue
            if pending is not None:
                found.append((img.cstr(pending), slot, value))
                pending = None
    return found


if __name__ == "__main__":
    func = int(sys.argv[1], 16)
    needle = sys.argv[2] if len(sys.argv) > 2 else ""
    for name, slot, value in pairs(func):
        if needle and needle not in name:
            continue
        print(f"{value:#x}  slot={slot:#x}  {name[:64]}")
