#!/usr/bin/env python3
"""Rough PS2 map stats from mapout.vpa (MAP1, section 1 = 3D map render file,
docs/formats/field.md "Map file reader"): record kinds, and for kind-1
records the word at data +0xc (hypothesis: vertex count), plus a VIF scan
counting UNPACK elements (num field) of V3/V4 unpacks per record.
Usage: ps2map.py mapout.vpa...
"""
import collections
import json
import struct
import sys


def vif_scan(b, o, end):
    """Walk a VIF stream; return counts of unpack elements by (vn, vl)."""
    c = collections.Counter()
    while o + 4 <= end:
        w = struct.unpack_from('<I', b, o)[0]
        cmd = (w >> 24) & 0x7f
        num = (w >> 16) & 0xff
        imm = w & 0xffff
        o += 4
        if cmd >= 0x60:
            vn, vl = (cmd >> 2) & 3, cmd & 3
            n = num or 256
            bits = (32 >> vl) * (vn + 1)
            size = (bits * n + 31) // 32 * 4
            c[(vn, vl)] += n
            o += size
        elif cmd in (0x50, 0x51):  # DIRECT / DIRECTHL
            o += (imm or 0x10000) * 16
        elif cmd == 0x30 or cmd == 0x31:  # STROW / STCOL
            o += 16
        elif cmd == 0x20:  # STMASK
            o += 4
        elif cmd == 0x4a:  # MPG
            o += (num or 256) * 8
        elif cmd in (0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x10, 0x11, 0x13, 0x14, 0x15, 0x17):
            pass
        else:
            c['bad'] += 1
            break
    return c


def parse(path):
    b = open(path, 'rb').read()
    s = struct.unpack_from('<I', b, 0x14)[0]
    if b[:4] != b'MAP1' or not s:
        return {'file': path, 'error': 'no section 1'}
    magic, ver, _, n = struct.unpack_from('<4I', b, s)
    o = s + 0x40
    kinds = collections.Counter()
    hdr_verts = 0
    unp = collections.Counter()
    for _ in range(n):
        k, ln = struct.unpack_from('<II', b, o)
        kinds[k] += 1
        d = o + 0x40
        if k == 1:
            hdr_verts += struct.unpack_from('<I', b, d + 0xc)[0]
            unp += vif_scan(b, d + 0x40, d + ln)
        o += 0x40 + ln
    return {'file': path, 'records': dict(kinds), 'models': kinds.get(1, 0), 'hdr_verts': hdr_verts,
            'unpack': {str(k): v for k, v in unp.items()}}


if __name__ == '__main__':
    for p in sys.argv[1:]:
        print(json.dumps(parse(p)))
