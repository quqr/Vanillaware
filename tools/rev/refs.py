import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nso import Image, BASE
from vc import load, body

img = Image()
index = load()

BASE_RE = re.compile(r"c->x\[(\d+)\] = \(uint64_t\)\((0x7000[0-9a-f]{3}000)ull\);")
ADD_RE = re.compile(r"c->x\[(\d+)\] = \(uint64_t\)\(c->x\[(\d+)\] \+ (0x[0-9a-f]+)ull\);")
SET_RE = re.compile(r"c->x\[(\d+)\] = \(uint64_t\)\(0x7000[0-9a-f]{5}([0-9a-f]{3})ull\);")

STRING_RE = re.compile(r"c->x\[(\d+)\] = \(uint64_t\)\((0x7000a[0-9a-f]{5})ull\);")


def refs(text):
    """Resolve ADRP+ADD pairs into absolute addresses referenced by the function."""
    bases = {}
    out = []
    for line in text.split("\n"):
        m = BASE_RE.search(line)
        if m:
            bases[m.group(1)] = int(m.group(2), 16)
            continue
        m = ADD_RE.search(line)
        if m and m.group(1) == m.group(2) and m.group(1) in bases:
            out.append(bases[m.group(1)] + int(m.group(3), 16))
            del bases[m.group(1)]
    return out


def main():
    wanted = sys.argv[1] if len(sys.argv) > 1 else None
    hits = []
    for addr in index:
        text = body(index, addr)
        if "0x7000" not in text:
            continue
        for target in refs(text):
            if not (BASE + img.rodata[0] <= target < BASE + img.rodata[0] + img.rodata[1]):
                continue
            try:
                name = img.cstr(target)
            except ValueError:
                continue
            if not name or len(name) < 3 or any(ord(ch) < 0x20 for ch in name):
                continue
            if wanted and wanted.lower() not in name.lower():
                continue
            hits.append((addr, target, name))
    for addr, target, name in hits[:400]:
        print(f"{addr:#x} {target:#x} {name}")
    print("total", len(hits))


if __name__ == "__main__":
    main()
