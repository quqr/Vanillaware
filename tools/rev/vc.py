import glob
import os
import pickle
import re
import sys

ROOT = r"F:/Codes/UNICORN OVERLORD/gen/main"
CACHE = os.path.join(os.path.dirname(__file__), "index.pkl")

FN_DEF = re.compile(r"^void (f_[0-9a-f]+)\(Cpu \*c\) \{\s*$", re.M)


def build():
    index = {}
    for path in sorted(glob.glob(os.path.join(ROOT, "main_*.c"))):
        lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
        current = None
        start = 0
        for number, line in enumerate(lines):
            m = FN_DEF.match(line)
            if m:
                if current:
                    index[current] = (path, start, number - 1)
                current = int(m.group(1)[2:], 16)
                start = number
        if current:
            index[current] = (path, start, len(lines) - 1)
    return index


def load():
    if os.path.exists(CACHE):
        return pickle.load(open(CACHE, "rb"))
    index = build()
    pickle.dump(index, open(CACHE, "wb"))
    return index


def body(index, addr):
    path, start, end = index[addr]
    lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
    return "\n".join(lines[start:end + 1])


def main():
    index = load()
    if len(sys.argv) == 1:
        print("functions", len(index))
        return
    addr = int(sys.argv[1], 16)
    if addr < 0x7000000000:
        addr += 0x7000000000
    if addr not in index:
        print("not found", hex(addr))
        return
    text = body(index, addr)
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    print(f"// {hex(addr)}  {index[addr][0]}:{index[addr][1] + 1}  lines={text.count(chr(10)) + 1}")
    if limit:
        print("\n".join(text.split("\n")[:limit]))
    else:
        print(text)


if __name__ == "__main__":
    main()
