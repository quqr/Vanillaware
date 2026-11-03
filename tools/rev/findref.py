import glob
import os
import re
import sys

BASE = 0x7000000000
ROOT = os.environ.get("VW_GEN_MAIN", "")

FN_DEF = re.compile(r"^void (f_[0-9a-f]+)\(Cpu \*c\) \{\s*$", re.M)
BASE_RE = re.compile(r"c->x\[(\d+)\] = \(uint64_t\)\((0x700[0-9a-f]{4}000)ull\);")
ADD_RE = re.compile(r"c->x\[(\d+)\] = \(uint64_t\)\(c->x\[(\d+)\] \+ (0x[0-9a-f]+)ull\);")


def main():
    if not ROOT:
        sys.exit("set VW_GEN_MAIN to the decompiled main directory")
    wanted = int(sys.argv[1], 16)
    if wanted < BASE:
        wanted += BASE
    hits = 0
    for path in sorted(glob.glob(os.path.join(ROOT, "main_*.c"))):
        lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
        fn = 0
        bases = {}
        for number, line in enumerate(lines):
            m = FN_DEF.match(line)
            if m:
                fn = int(m.group(1)[2:], 16)
                bases = {}
                continue
            m = BASE_RE.search(line)
            if m:
                bases[m.group(1)] = int(m.group(2), 16)
                continue
            m = ADD_RE.search(line)
            if not m or m.group(1) != m.group(2) or m.group(1) not in bases:
                continue
            target = bases.pop(m.group(1)) + int(m.group(3), 16)
            if target != wanted:
                continue
            hits += 1
            print(f"=== {fn:#x}  {path}:{number + 1}")
            print("\n".join(lines[max(0, number - 8):number + 10]))
            print()
    print("total", hits)


if __name__ == "__main__":
    main()
