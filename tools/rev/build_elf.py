import struct
import sys

sys.path.insert(0, "F:/Codes/Vanillaware/tmp/rev")
from nso import Image, BASE

OUT = r"F:/Codes/Vanillaware/tmp/rev/uo.elf"

img = Image()

segs = [
    (BASE + img.text[0], img.text[1], 0x5),      # R+X
    (BASE + img.rodata[0], img.rodata[1], 0x4),  # R
    (BASE + img.data[0], img.data[1] + img.bss, 0x6),  # R+W
]

ehdr_size = 64
phdr_size = 56
phnum = len(segs)
offset = ehdr_size + phdr_size * phnum
offset = (offset + 0xFFF) & ~0xFFF

phdrs = []
blobs = []
cursor = offset
for vaddr, size, flags in segs:
    file_size = size
    if vaddr == BASE + img.data[0]:
        file_size = img.data[1]  # bss is not in the file
    phdrs.append(struct.pack("<IIQQQQQQ", 1, flags, cursor, vaddr, vaddr, file_size, size, 0x1000))
    blobs.append((cursor, img.image[vaddr - BASE:vaddr - BASE + file_size]))
    cursor += (file_size + 0xFFF) & ~0xFFF
    cursor = (cursor + 0xFFF) & ~0xFFF

ident = b"\x7fELF" + bytes([2, 1, 1, 0]) + b"\0" * 8
ehdr = ident + struct.pack(
    "<HHIQQQIHHHHHH",
    2, 183, 1, 0, ehdr_size, 0, 0,
    ehdr_size, phdr_size, phnum, 64, 0, 0,
)

out = bytearray(cursor)
out[0:ehdr_size] = ehdr
for index, phdr in enumerate(phdrs):
    out[ehdr_size + index * phdr_size:ehdr_size + (index + 1) * phdr_size] = phdr
for start, blob in blobs:
    out[start:start + len(blob)] = blob

open(OUT, "wb").write(out)
print("wrote", OUT, hex(len(out)))
for vaddr, size, flags in segs:
    print(f"  seg {vaddr:#x} file={size:#x} flags={flags}")
