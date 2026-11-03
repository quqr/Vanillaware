import struct

BASE = 0x7000000000
NSO = r"F:/Codes/UNICORN OVERLORD/extract/160d1db236ac9d15b509ef661bf18b7b_NCA/0/main"

DT_NULL, DT_NEEDED, DT_PLTRELSZ, DT_HASH, DT_STRTAB, DT_SYMTAB, DT_RELA, DT_RELASZ = 0, 1, 2, 4, 5, 6, 7, 8
DT_STRSZ, DT_JMPREL, DT_RELACOUNT = 10, 23, 0x6FFFFFF9
R_RELATIVE = 1027


def lz4_block(src, out_size):
    dst = bytearray()
    i = 0
    n = len(src)
    while i < n:
        token = src[i]
        i += 1
        literals = token >> 4
        if literals == 15:
            while True:
                b = src[i]
                i += 1
                literals += b
                if b != 255:
                    break
        dst += src[i:i + literals]
        i += literals
        if i >= n:
            break
        offset = src[i] | (src[i + 1] << 8)
        i += 2
        match_len = token & 0xF
        if match_len == 15:
            while True:
                b = src[i]
                i += 1
                match_len += b
                if b != 255:
                    break
        match_len += 4
        start = len(dst) - offset
        if start < 0:
            raise ValueError("bad match offset")
        for k in range(match_len):
            dst.append(dst[start + k])
        if len(dst) > out_size + 16:
            raise ValueError("overrun")
    if len(dst) != out_size:
        raise ValueError(f"size mismatch {len(dst)} != {out_size}")
    return bytes(dst)


class Image:
    def __init__(self):
        raw = open(NSO, "rb").read()
        assert raw[:4] == b"NSO0"
        flags = struct.unpack_from("<I", raw, 0xC)[0]
        segs = []
        for index in range(3):
            file_off, mem_off, dec_size = struct.unpack_from("<III", raw, 0x10 + index * 0x10)
            comp_size = struct.unpack_from("<I", raw, 0x60 + index * 4)[0]
            blob = raw[file_off:file_off + comp_size]
            data = lz4_block(blob, dec_size) if flags & (1 << index) else blob[:dec_size]
            segs.append((mem_off, data))
        self.bss = struct.unpack_from("<I", raw, 0x3C)[0]
        self.text = (segs[0][0], len(segs[0][1]))
        self.rodata = (segs[1][0], len(segs[1][1]))
        self.data = (segs[2][0], len(segs[2][1]))
        size = (segs[2][0] + len(segs[2][1]) + self.bss + 0xFFF) & ~0xFFF
        self.image = bytearray(size)
        for off, data in segs:
            self.image[off:off + len(data)] = data
        self._relocate()

    def _relocate(self):
        mod0 = struct.unpack_from("<I", self.image, 4)[0]
        rel = lambda i: mod0 + struct.unpack_from("<i", self.image, mod0 + 4 + i * 4)[0]
        dynamic = rel(0)
        dyn = {}
        off = dynamic
        while True:
            tag, val = (struct.unpack_from("<Q", self.image, off)[0],
                        struct.unpack_from("<Q", self.image, off + 8)[0])
            off += 16
            if tag == DT_NULL:
                break
            dyn.setdefault(tag, val)
        self.dyn = dyn
        self.reloctl = 0
        for base_tag, size_tag in ((DT_RELA, DT_RELASZ), (DT_JMPREL, DT_PLTRELSZ)):
            if base_tag not in dyn:
                continue
            base = dyn[base_tag]
            count = dyn.get(size_tag, 0) // 24
            for i in range(count):
                r_off, r_info, r_add = struct.unpack_from("<QQq", self.image, base + i * 24)
                kind = r_info & 0xFFFFFFFF
                if kind == R_RELATIVE:
                    struct.pack_into("<Q", self.image, r_off, (BASE + r_add) & 0xFFFFFFFFFFFFFFFF)
                    self.reloctl += 1

    def read(self, addr, size):
        off = addr - BASE
        return bytes(self.image[off:off + size])

    def u32(self, addr):
        return struct.unpack_from("<I", self.image, addr - BASE)[0]

    def i32(self, addr):
        return struct.unpack_from("<i", self.image, addr - BASE)[0]

    def u64(self, addr):
        return struct.unpack_from("<Q", self.image, addr - BASE)[0]

    def f32(self, addr):
        return struct.unpack_from("<f", self.image, addr - BASE)[0]

    def cstr(self, addr):
        off = addr - BASE
        end = self.image.index(b"\0", off)
        return self.image[off:end].decode("utf-8", "replace")

    def is_code(self, addr):
        return BASE + self.text[0] <= addr < BASE + self.text[0] + self.text[1]


if __name__ == "__main__":
    img = Image()
    print(f"text {BASE + img.text[0]:#x}+{img.text[1]:#x}")
    print(f"rodata {BASE + img.rodata[0]:#x}+{img.rodata[1]:#x}")
    print(f"data {BASE + img.data[0]:#x}+{img.data[1]:#x}")
    print("relocations applied:", img.reloctl)
    print("FnMoneResource ->", img.cstr(0x7000a86065))
    print("desc ptr 0x70028c9020 ->", hex(img.u64(0x70028c9020)))
