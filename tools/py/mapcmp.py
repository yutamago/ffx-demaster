#!/usr/bin/env python3
"""Field/battle map: PS2 mapout.vpa geometry (VIF V3-32 positions, V2-16 UVs)
vs the HD map glb. Prints position-set overlap (HD y negated) per map.
Usage: mapcmp.py <ps2 mapout.vpa> <hd .glb>...  (pairs)
"""
import json
import os
import struct
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import glbstat  # noqa: E402


def ps2_geom(path):
    b = open(path, 'rb').read()
    s = struct.unpack_from('<I', b, 0x14)[0]
    n = struct.unpack_from('<I', b, s + 0xc)[0]
    o = s + 0x40
    P, UV, models = [], [], 0
    for _ in range(n):
        k, ln = struct.unpack_from('<II', b, o)
        d = o + 0x40
        if k == 1:
            models += 1
            q, end = d + 0x40, d + ln
            while q + 4 <= end:
                w = struct.unpack_from('<I', b, q)[0]
                cmd, num, imm = (w >> 24) & 0x7f, ((w >> 16) & 0xff) or 256, w & 0xffff
                q += 4
                if cmd >= 0x60:
                    vn, vl = (cmd >> 2) & 3, cmd & 3
                    size = ((32 >> vl) * (vn + 1) * num + 31) // 32 * 4
                    if (vn, vl) == (2, 0):
                        P += list(struct.iter_unpack('<3f', b[q:q + 12 * num]))
                    elif (vn, vl) == (1, 1):
                        UV += list(struct.iter_unpack('<2h', b[q:q + 4 * num]))
                    q += size
                elif cmd in (0x50, 0x51):
                    q += (imm or 0x10000) * 16
                elif cmd in (0x30, 0x31):
                    q += 16
                elif cmd == 0x20:
                    q += 4
                elif cmd == 0x4a:
                    q += num * 8
        o += 0x40 + ln
    return np.array(P).reshape(-1, 3), np.array(UV, float).reshape(-1, 2), models


def hd_geom(path):
    js, bb = glbstat.load(path)
    H, tris = [], 0
    for m in js['meshes']:
        for p in m['primitives']:
            H += glbstat.accessor(js, bb, p['attributes']['POSITION'])[0]
            tris += js['accessors'][p['indices']]['count'] // 3 if 'indices' in p else 0
    return np.array(H).reshape(-1, 3), tris, len(js['meshes'])


def hist(lengths, scale=100):
    import collections
    return collections.Counter(np.round(np.asarray(lengths) * scale).astype(np.int64).tolist())


def ps2_edges(P):
    a = np.linalg.norm(P[1:] - P[:-1], axis=1)
    b = np.linalg.norm(P[2:] - P[:-2], axis=1)
    e = np.concatenate([a, b])
    return e[e > 0]


def hd_edges(path):
    js, bb = glbstat.load(path)
    out = []
    for m in js['meshes']:
        for p in m['primitives']:
            pos = np.array(glbstat.accessor(js, bb, p['attributes']['POSITION'])[0])
            ix = np.array(glbstat.accessor(js, bb, p['indices'])[0]).reshape(-1, 3)
            for i, j in ((0, 1), (1, 2), (0, 2)):
                e = np.linalg.norm(pos[ix[:, i]] - pos[ix[:, j]], axis=1)
                out.append(e)
    e = np.concatenate(out) if out else np.zeros(0)
    return e[e > 0]


def key(a, scale=50):  # 2 cm grid
    return set(map(tuple, np.round(a * scale).astype(np.int64)))


def main(args):
    for vpa, glb in zip(args[::2], args[1::2]):
        P, UV, models = ps2_geom(vpa)
        H, tris, meshes = hd_geom(glb)
        H2 = H * np.array([1, -1, 1])
        kp, kh = key(P), key(H2)
        r = {'map': os.path.basename(glb), 'ps2_models': models, 'ps2_strip_verts': len(P),
             'ps2_uniq_pos': len(kp), 'hd_meshes': meshes, 'hd_verts': len(H), 'hd_tris': tris,
             'hd_uniq_pos': len(kh),
             'hd_pos_in_ps2': round(len(kh & kp) / max(1, len(kh)), 3),
             'ps2_pos_in_hd': round(len(kh & kp) / max(1, len(kp)), 3)}
        hp, hh = hist(ps2_edges(P)), hist(hd_edges(glb))
        r['edge_overlap'] = round(sum((hp & hh).values()) / max(1, min(sum(hp.values()), sum(hh.values()))), 3)
        print(json.dumps(r), flush=True)


if __name__ == '__main__':
    main(sys.argv[1:])
