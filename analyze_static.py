from __future__ import annotations

import hashlib
import struct
import sys
from pathlib import Path


def u16(b: bytes, o: int) -> int:
    return struct.unpack_from("<H", b, o)[0]


def u32(b: bytes, o: int) -> int:
    return struct.unpack_from("<I", b, o)[0]


def u64(b: bytes, o: int) -> int:
    return struct.unpack_from("<Q", b, o)[0]


def strings(data: bytes, minlen: int = 4):
    start = None
    for i, c in enumerate(data):
        if 0x20 <= c <= 0x7e:
            if start is None:
                start = i
        else:
            if start is not None and i - start >= minlen:
                yield start, data[start:i].decode("ascii", "replace")
            start = None
    if start is not None and len(data) - start >= minlen:
        yield start, data[start:].decode("ascii", "replace")


def inspect_file(path: Path):
    data = path.read_bytes()
    print(f"FILE {path}")
    print(f"SIZE {len(data)}")
    print(f"SHA256 {hashlib.sha256(data).hexdigest().upper()}")
    if data[:2] == b"MZ":
        inspect_pe(data)
    else:
        inspect_fbmod(data)


def inspect_pe(data: bytes):
    pe = u32(data, 0x3c)
    assert data[pe:pe + 4] == b"PE\0\0"
    coff = pe + 4
    machine, nsects = struct.unpack_from("<HH", data, coff)
    timestamp = u32(data, coff + 4)
    opt_size = u16(data, coff + 16)
    characteristics = u16(data, coff + 18)
    opt = coff + 20
    magic = u16(data, opt)
    ep = u32(data, opt + 16)
    imagebase = u64(data, opt + 24) if magic == 0x20b else u32(data, opt + 28)
    size_image = u32(data, opt + 56)
    size_headers = u32(data, opt + 60)
    subsystem = u16(data, opt + 68)
    n_dirs = u32(data, opt + (108 if magic == 0x20b else 92))
    dirs_at = opt + (112 if magic == 0x20b else 96)
    print(f"PE offset=0x{pe:X} machine=0x{machine:04X} sections={nsects} timestamp=0x{timestamp:08X}")
    print(f"optional_magic=0x{magic:04X} entry_rva=0x{ep:X} imagebase=0x{imagebase:X} size_image=0x{size_image:X} headers=0x{size_headers:X} subsystem={subsystem} characteristics=0x{characteristics:04X}")
    sections = []
    sh = opt + opt_size
    for i in range(nsects):
        o = sh + i * 40
        name = data[o:o + 8].split(b"\0", 1)[0].decode("ascii", "replace")
        vs, va, rs, rp = struct.unpack_from("<IIII", data, o + 8)
        chars = u32(data, o + 36)
        sections.append((name, va, max(vs, rs), rp, rs))
        print(f"SECTION {name:8} RVA=0x{va:08X} VS=0x{vs:X} RAW=0x{rs:X} FILE=0x{rp:X} FLAGS=0x{chars:08X}")

    def rva_to_off(rva):
        if rva < size_headers:
            return rva
        for name, va, span, rp, rs in sections:
            if va <= rva < va + span:
                return rp + (rva - va)
        return None

    if n_dirs > 1:
        imp_rva, imp_size = struct.unpack_from("<II", data, dirs_at + 8)
        print(f"IMPORT_DIRECTORY RVA=0x{imp_rva:X} SIZE=0x{imp_size:X}")
        off = rva_to_off(imp_rva) if imp_rva else None
        imports = []
        if off is not None:
            for _ in range(256):
                if off + 20 > len(data):
                    break
                oft, tds, fchain, name_rva, ft = struct.unpack_from("<IIIII", data, off)
                if not any((oft, tds, fchain, name_rva, ft)):
                    break
                no = rva_to_off(name_rva)
                if no is None:
                    break
                end = data.find(b"\0", no)
                dll = data[no:end].decode("ascii", "replace")
                imports.append(dll)
                off += 20
        print("IMPORT_DLLS " + ", ".join(imports))
    print("ASCII_MARKERS")
    for term in (b"Invalid magic: expected FROSTY header", b"Decompression error: ", b"src\\fbmod.rs", b"src\\extract.rs", b"resource_index", b"original_size", b"logical_offset", b"Warn: CAS decompress '", b"zstd: ", b"zstd init: ", b"zlib: ", b"lz4: ", b"corrupt deflate stream", b"--input", b"--output"):
        off = data.lower().find(term.lower())
        if off >= 0:
            print(f"  0x{off:08X} {term.decode('ascii', 'replace')}")


def inspect_fbmod(data: bytes):
    print(f"MAGIC {data[:8]!r}")
    version = u32(data, 8)
    data_offset = u64(data, 12)
    data_count = u32(data, 20)
    profile_len = data[24]
    profile = data[25:25 + profile_len].decode("ascii", "replace")
    cursor = 25 + profile_len
    game_version = u32(data, cursor)
    cursor += 4
    details = []
    for _ in range(6):
        end = data.find(b"\0", cursor)
        if end < 0:
            raise ValueError("unterminated metadata string")
        details.append(data[cursor:end].decode("utf-8", "replace"))
        cursor = end + 1
    resource_count = u32(data, cursor)
    cursor += 4
    print(f"HEADER version={version} data_offset=0x{data_offset:X} data_count={data_count} profile_len={profile_len} profile={profile!r} game_version={game_version}")
    print(f"DETAILS title={details[0]!r} author={details[1]!r} category={details[2]!r} mod_version={details[3]!r} description={details[4]!r} link={details[5]!r}")
    print(f"RESOURCE_COUNT {resource_count} records_start=0x{cursor:X} target_manifest=0x{data_offset:X}")
    type_names = {0: "embedded", 1: "ebx", 2: "res", 3: "chunk", 4: "bundle"}
    resources = []
    for i in range(resource_count):
        start = cursor
        typ = data[cursor]
        cursor += 1
        ridx = struct.unpack_from("<i", data, cursor)[0]
        cursor += 4
        name_end = data.find(b"\0", cursor)
        name = data[cursor:name_end].decode("utf-8", "replace")
        cursor = name_end + 1
        sha1 = "-"
        size = -1
        flags = 0
        handler = 0
        user_data = ""
        if ridx != -1:
            sha1 = data[cursor:cursor + 20].hex().upper()
            cursor += 20
            size = struct.unpack_from("<q", data, cursor)[0]
            cursor += 8
            flags = data[cursor]
            cursor += 1
            handler = struct.unpack_from("<i", data, cursor)[0]
            cursor += 4
            user_end = data.find(b"\0", cursor)
            user_data = data[cursor:user_end].decode("utf-8", "replace")
            cursor = user_end + 1
        bundle_count = struct.unpack_from("<i", data, cursor)[0]
        cursor += 4
        bundles = list(struct.unpack_from(f"<{bundle_count}i", data, cursor)) if bundle_count else []
        cursor += bundle_count * 4
        extra = ""
        if typ == 2:
            res_type = u32(data, cursor); res_rid = u64(data, cursor + 4); meta_len = struct.unpack_from("<i", data, cursor + 12)[0]
            if meta_len < 0:
                raise ValueError(f"negative RES metadata length at 0x{cursor + 12:X}")
            cursor += 16
            meta = data[cursor:cursor + meta_len]
            cursor += meta_len
            extra = f" res_type=0x{res_type:X} res_rid=0x{res_rid:X} meta_len={meta_len}"
        elif typ == 3:
            vals = struct.unpack_from("<IIIIii", data, cursor)
            cursor += 24
            extra = " chunk=" + "/".join(f"0x{x:X}" for x in vals)
        elif typ == 4:
            # Writer stores bundle name again, followed by its super-bundle hash.
            b_end = data.find(b"\0", cursor)
            bundle_name = data[cursor:b_end].decode("utf-8", "replace")
            cursor = b_end + 1
            super_hash = struct.unpack_from("<i", data, cursor)[0]
            cursor += 4
            extra = f" bundle_name={bundle_name!r} super_hash=0x{super_hash & 0xFFFFFFFF:08X}"
        resources.append((i, type_names.get(typ, f"type{typ}"), ridx, name, sha1, size, flags, handler, user_data, bundles, extra, start, cursor))
    print(f"RESOURCE_RECORDS_END 0x{cursor:X} delta_to_manifest={data_offset - cursor}")
    resource_type_counts = {name: sum(1 for resource in resources if resource[1] == name)
                            for name in ("embedded", "ebx", "res", "chunk", "bundle")}
    resource_type_counts["total"] = len(resources)
    print("RESOURCE_TYPE_COUNTS " + " ".join(f"{name}={count}" for name, count in resource_type_counts.items()))
    for i, typ, ridx, name, sha1, size, flags, handler, user_data, bundles, extra, start, end in resources:
        print(f"  [{i:02}] {typ:8} @0x{start:X}..0x{end:X} idx={ridx:3} name={name!r} sha1={sha1} size={size:8} flags=0x{flags:02X} handler=0x{handler & 0xFFFFFFFF:08X} bundles={bundles}{extra}")
    table_start = data_offset
    payload_start = data_offset + data_count * 16
    print(f"MANIFEST table=0x{table_start:X} payload=0x{payload_start:X} file_size=0x{len(data):X}")
    manifests = []
    for i in range(data_count):
        off, sz = struct.unpack_from("<QQ", data, table_start + i * 16)
        abs_start = payload_start + off
        blob = data[abs_start:abs_start + sz]
        manifests.append((i, off, sz, abs_start, blob))
        print(f"  [{i:02}] rel=0x{off:X} size=0x{sz:X} abs=0x{abs_start:X}..0x{abs_start + sz:X} sha256={hashlib.sha256(blob).hexdigest()[:16].upper()} sig={blob[:8].hex(' ').upper()}")
    print(f"MANIFEST_END 0x{payload_start + max((m[1] + m[2] for m in manifests), default=0):X} trailing={len(data) - (payload_start + max((m[1] + m[2] for m in manifests), default=0))}")
    print("RESOURCE_MANIFEST_CORRELATION")
    cas_selector_counts = {}
    for i, typ, ridx, name, sha1, size, flags, handler, user_data, bundles, extra, start, end in resources:
        if ridx < 0 or ridx >= len(manifests):
            continue
        _, rel, stored_size, abs_start, blob = manifests[ridx]
        sha_match = (sha1 == hashlib.sha1(blob).hexdigest().upper())
        block_parts = []
        if typ in ("ebx", "res", "chunk"):
            expected_output = size
            if typ == "chunk":
                try:
                    expected_output = int(extra.removeprefix(" chunk=").split("/")[3], 16)
                except (IndexError, ValueError):
                    expected_output = -1
            at = 0
            while at + 8 <= len(blob):
                uncomp = int.from_bytes(blob[at:at + 4], "big")
                codec_byte, size_flags = blob[at + 4], blob[at + 5]
                codec = codec_byte & 0x7F
                tag = (codec_byte << 8) | size_flags
                csize = ((size_flags & 0x0F) << 16) | int.from_bytes(blob[at + 6:at + 8], "big")
                stored = uncomp if codec == 0 and csize == 0 else csize
                if uncomp == 0 or at + 8 + stored > len(blob):
                    break
                block_parts.append((uncomp, codec, tag, csize, stored))
                key = (typ, codec)
                cas_selector_counts[key] = cas_selector_counts.get(key, 0) + 1
                at += 8 + stored
            block_ok = at == len(blob)
            if block_parts:
                total_uncomp = sum(x[0] for x in block_parts)
                shape = ",".join(f"{u:#x}/codec={codec}/tag={tag:#06x}/packed={c:#x}/stored={stored:#x}" for u, codec, tag, c, stored in block_parts)
                print(f"  [{i:02}] idx={ridx:02} type={typ} stored={stored_size} declared_size={size} expected_output={expected_output} sha1_match={sha_match} CAS_blocks={len(block_parts)} total_uncompressed={total_uncomp} exact={block_ok and total_uncomp == expected_output} blocks={shape}")
            else:
                print(f"  [{i:02}] idx={ridx:02} type={typ} stored={stored_size} declared_size={size} expected_output={expected_output} sha1_match={sha_match} CAS_blocks=unparsed")
        else:
            print(f"  [{i:02}] idx={ridx:02} stored={stored_size} declared_size={size} sha1_match={sha_match} type={typ}")
    print("CAS_SELECTOR_COUNTS " + " ".join(f"{kind}:{selector}={count}" for (kind, selector), count in sorted(cas_selector_counts.items())))
    for start, end in ((0, 0x100), (0x820, 0x920)):
        if start >= len(data):
            continue
        end = min(end, len(data))
        print(f"HEX 0x{start:04X}..0x{end:04X}")
        for off in range(start, end, 16):
            chunk = data[off:min(off + 16, end)]
            asc = "".join(chr(c) if 0x20 <= c <= 0x7e else "." for c in chunk)
            print(f"  {off:08X}  {chunk.hex(' ').upper():47}  {asc}")
    print("PRINTABLE_STRINGS_BEFORE_MANIFEST")
    for off, s in strings(data[:data_offset], 8):
        if any(c.isalpha() for c in s) and len(s) < 180:
            print(f"  0x{off:08X} {s}")


if __name__ == "__main__":
    for arg in sys.argv[1:]:
        inspect_file(Path(arg))
