#!/usr/bin/env python3
"""Size, section and import numbers for one built target.

    measure.py <name>.elf <name>.exe

Reads the linked ELF (before elf2e32) for the section breakdown and the imports,
and the E32 image's own header for the code/data sizes the loader sees. The E32
body is compressed with Symbian's own LZ77+Huffman ("deflate" UID 0x101F7AFC),
which is not RFC 1951, so nothing below decompresses it: every import number
comes from the ELF, by the same rule `E32ImportSection::from_elf` applies —
`DT_NEEDED` DSOs, and the relocations against undefined symbols grouped by the
symbol's version, which for a Symbian DSO is the defining DLL.

Output is one `key=value` line per fact, so a shell can diff two runs.
"""

import struct
import sys

SHT_DYNAMIC = 6


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


class Elf:
    def __init__(self, path):
        self.b = open(path, "rb").read()
        b = self.b
        if b[:4] != b"\x7fELF":
            raise SystemExit(f"{path}: not an ELF")
        shoff, shentsize, shnum, shstrndx = u32(b, 0x20), u16(b, 0x2E), u16(b, 0x30), u16(b, 0x32)
        raw = []
        for i in range(shnum):
            o = shoff + i * shentsize
            raw.append(dict(name=u32(b, o), type=u32(b, o + 4), flags=u32(b, o + 8),
                            addr=u32(b, o + 12), off=u32(b, o + 16), size=u32(b, o + 20),
                            link=u32(b, o + 24), info=u32(b, o + 28), entsize=u32(b, o + 36)))
        strtab = raw[shstrndx]
        self.sections = {}
        self.order = []
        for s in raw:
            s["sname"] = self.cstr(strtab["off"] + s["name"])
            self.sections[s["sname"]] = s
            self.order.append(s)

    def cstr(self, off):
        end = self.b.index(b"\0", off)
        return self.b[off:end].decode("latin1")

    def data(self, name):
        s = self.sections.get(name)
        return b"" if s is None else self.b[s["off"]:s["off"] + s["size"]]

    def size(self, name):
        s = self.sections.get(name)
        return 0 if s is None else s["size"]

    # -- dynamic linking -------------------------------------------------

    def dynstr_off(self):
        return self.sections[".dynstr"]["off"]

    def needed(self):
        out = []
        d = self.data(".dynamic")
        base = self.dynstr_off()
        for o in range(0, len(d), 8):
            tag, val = struct.unpack_from("<Ii", d, o)
            if tag == 0:
                break
            if tag == 1:  # DT_NEEDED
                out.append(self.cstr(base + val))
        return out

    def dynsyms(self):
        """(name, shndx) per .dynsym entry."""
        d = self.data(".dynsym")
        base = self.dynstr_off()
        out = []
        for o in range(0, len(d), 16):
            nameoff, _val, _sz, _info, _other, shndx = struct.unpack_from("<IIIBBH", d, o)
            out.append((self.cstr(base + nameoff), shndx))
        return out

    def versym(self):
        d = self.data(".gnu.version")
        return [u16(d, o) for o in range(0, len(d), 2)]

    def verneed(self):
        """version index -> the DSO/DLL the symbol comes from."""
        d = self.data(".gnu.version_r")
        base = self.dynstr_off()
        out = {}
        o = 0
        while o < len(d):
            _ver, cnt, file_off, aux, nxt = struct.unpack_from("<HHIII", d, o)
            filename = self.cstr(base + file_off)
            a = o + aux
            for _ in range(cnt):
                _hash, _flags, other, name_off, anext = struct.unpack_from("<IHHII", d, a)
                out[other] = (self.cstr(base + name_off), filename)
                if anext == 0:
                    break
                a += anext
            if nxt == 0:
                break
            o += nxt
        return out

    def import_slots(self):
        """{dll: (distinct symbol names, relocation slot count)}"""
        syms = self.dynsyms()
        vs = self.versym()
        vn = self.verneed()
        per = {}
        for sec in (".rel.dyn", ".rel.plt"):
            d = self.data(sec)
            for o in range(0, len(d), 8):
                _off, info = struct.unpack_from("<II", d, o)
                idx = info >> 8
                if idx == 0 or idx >= len(syms):
                    continue
                name, shndx = syms[idx]
                if shndx != 0:  # defined here, not an import
                    continue
                if name == "__cxa_pure_virtual":
                    continue
                v = vs[idx] if idx < len(vs) else 0
                dll = vn.get(v, ("?", "?"))[0]
                names, slots = per.setdefault(dll, (set(), 0))
                names.add(name)
                per[dll] = (names, slots + 1)
        return per


E32_FIELDS = [
    ("uid1", 0x00), ("uid2", 0x04), ("uid3", 0x08),
    ("compression_type", 0x1C), ("flags", 0x2C), ("code_size", 0x30),
    ("data_size", 0x34), ("heap_size_min", 0x38), ("heap_size_max", 0x3C),
    ("stack_size", 0x40), ("bss_size", 0x44), ("entry_point", 0x48),
    ("dll_ref_table_count", 0x54), ("text_size", 0x60), ("code_offset", 0x64),
    ("data_offset", 0x68), ("import_offset", 0x6C),
    ("uncompressed_size", 0x7C),
]


def main():
    elf_path, exe_path = sys.argv[1], sys.argv[2]
    e = Elf(elf_path)

    print(f"elf_file_bytes={len(e.b)}")
    for s in (".text", ".emb_text", ".plt", ".rodata", ".constdata",
              ".ARM.extab", ".ARM.exidx", ".data", ".bss"):
        print(f"section{s}={e.size(s)}")
    alloc = sum(s["size"] for s in e.order if s["flags"] & 0x2 and s["type"] != 8)
    print(f"alloc_progbits_total={alloc}")

    needed = e.needed()
    per = e.import_slots()
    print(f"dt_needed={len(needed)} [{' '.join(needed)}]")
    dlls = sorted(per)
    print(f"import_dlls={len(dlls)}")
    total_sym = total_slot = 0
    for dll in dlls:
        names, slots = per[dll]
        total_sym += len(names)
        total_slot += slots
        print(f"import[{dll}]=ordinals:{len(names)} slots:{slots}")
    print(f"import_ordinals_total={total_sym}")
    print(f"import_slots_total={total_slot}")

    b = open(exe_path, "rb").read()
    print(f"e32_file_bytes={len(b)}")
    for name, off in E32_FIELDS:
        print(f"e32.{name}={u32(b, off)}")


if __name__ == "__main__":
    main()
