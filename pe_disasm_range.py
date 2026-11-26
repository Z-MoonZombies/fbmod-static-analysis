"""Disassemble a half-open PE RVA range with Capstone; never executes the PE."""
from __future__ import annotations
import struct
import sys
from pathlib import Path

from capstone import Cs, CS_ARCH_X86, CS_MODE_64


def u16(buf: bytes, off: int) -> int:
    return struct.unpack_from("<H", buf, off)[0]


def u32(buf: bytes, off: int) -> int:
    return struct.unpack_from("<I", buf, off)[0]


def u64(buf: bytes, off: int) -> int:
    return struct.unpack_from("<Q", buf, off)[0]


def main() -> None:
    if len(sys.argv) != 4:
        raise SystemExit("usage: python pe_disasm_range.py <exe> <start-rva> <end-rva>")
    data = Path(sys.argv[1]).read_bytes()
    pe = u32(data, 0x3C)
    if data[pe:pe + 4] != b"PE\0\0":
        raise SystemExit("not a PE file")
    coff = pe + 4
    nsec = u16(data, coff + 2)
    opt_size = u16(data, coff + 16)
    opt = coff + 20
    if u16(data, opt) != 0x20B:
        raise SystemExit("expected PE32+ image")
    image_base = u64(data, opt + 24)
    sh = opt + opt_size
    sections = []
    for i in range(nsec):
        off = sh + i * 40
        name = data[off:off + 8].split(b"\0", 1)[0].decode("ascii", "replace")
        virtual_size, rva, raw_size, raw_ptr = struct.unpack_from("<IIII", data, off + 8)
        sections.append((name, rva, max(virtual_size, raw_size), raw_ptr, raw_size))

    start = int(sys.argv[2], 0)
    end = int(sys.argv[3], 0)
    if end <= start:
        raise SystemExit("end-rva must be greater than start-rva")
    for _, rva, span, raw_ptr, raw_size in sections:
        if rva <= start < rva + span:
            offset = raw_ptr + start - rva
            available = min(end - start, raw_size - (start - rva))
            if available <= 0:
                raise SystemExit("requested RVA has no raw bytes")
            break
    else:
        raise SystemExit("start-rva is not mapped by a section")
    md = Cs(CS_ARCH_X86, CS_MODE_64)
    for ins in md.disasm(data[offset:offset + available], image_base + start):
        print(f"{ins.address - image_base:08X}: {ins.mnemonic:8} {ins.op_str}")


if __name__ == "__main__":
    main()
