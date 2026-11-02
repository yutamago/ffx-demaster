#!/usr/bin/env python3
"""Summarise a PS2 FFX .chr: textures, models, vertices, triangles, UV ranges.

Layout from ffx-decomp docs/formats/chara.md and crates/ffx-chara (mapping.rs,
chr.rs ch_model_create_packet, sys.rs ch_effect_packet_create_main).
Usage: ps2chr.py [--json] file.chr...
"""
import json
import struct
import sys


def u8(b, o): return b[o]
def u16(b, o): return struct.unpack_from('<H', b, o)[0]
def i16(b, o): return struct.unpack_from('<h', b, o)[0]
def u32(b, o): return struct.unpack_from('<I', b, o)[0]


def parse(path, want_points=False):
    b = open(path, 'rb').read()
    out = {'file': path, 'size': len(b)}
    n = u32(b, 4)
    entries = [(u32(b, 0x10 + 8 * i), u32(b, 0x14 + 8 * i)) for i in range(n)]
    out['entries'] = n
    # texture data (entry 1)
    tex = entries[1][0] if n > 1 else 0
    if tex:
        t = tex
        out['tex'] = {
            'version': u32(b, t + 0x10),
            'list': u16(b, t + 4), 'cluts': u16(b, t + 8), 'images': u16(b, t + 0xc),
        }
        imgs = []
        ip = t + u32(b, t + 0x1c)
        for k in range(u16(b, t + 0xc)):
            e = ip + 16 * k
            imgs.append({'psm': u16(b, e), 'w': u16(b, e + 8), 'h': u16(b, e + 0xa)})
        out['tex']['image_sizes'] = imgs
        lst = []
        lp = t + u32(b, t + 0x14)
        for k in range(u16(b, t + 4)):
            e = lp + 16 * k
            lst.append({'image': u8(b, e), 'clut': u8(b, e + 1), 'psm': u16(b, e + 2),
                        'w': u16(b, e + 4), 'h': u16(b, e + 6)})
        out['tex']['list_entries'] = lst
    h = entries[0][0]
    if not h:
        return out
    ver = u16(b, h + 4)
    nmodels = u16(b, h + 6)
    out['version'] = hex(ver)
    out['bones'] = u16(b, h + 0xa)
    esize = 24 if 0x831 <= ver < 0x1314 else 40
    mt = h + u32(b, h + 0x10)
    models = []
    for i in range(nmodels):
        e = mt + esize * i
        nv = u16(b, e + 4)
        ng = u16(b, e + 6)
        vp = h + u32(b, e + 8)
        gp = h + u32(b, e + 0x10)
        ncloth = u16(b, e + 0x18)
        tris = 0
        strips_pts = 0
        us, vs = [], []
        mats = set()
        used = set()
        pts = []
        vpts = []
        strips = []
        for g in range(ng):
            ge = gp + 12 * g
            word, mat = u8(b, ge), u8(b, ge + 1)
            mats.add(mat)
            cnt = u16(b, ge + 4)
            q = h + u32(b, ge + 8)
            if word & 1:
                for _ in range(cnt):
                    size = u8(b, q) << 4
                    point = q + u8(b, q + 1)
                    ns = u8(b, q + 2)
                    for j in range(ns):
                        L = u8(b, q + 4 + j)
                        tris += max(L - 2, 0)
                        strip = []
                        for s in range(L):
                            vi, ni, uu, vv = struct.unpack_from('<4H', b, point)
                            used.add(vi & 0x7fff)
                            us.append(uu); vs.append(vv)
                            if want_points:
                                pts.append((mat, uu, vv))
                                vpts.append((g, vi & 0x7fff, uu, vv))
                                strip.append(vi & 0x7fff)
                            point += 8
                            strips_pts += 1
                        if want_points:
                            strips.append(strip)
                    if size == 0:
                        break
                    q += size
            else:
                for t_ in range(cnt // 3):
                    for k in range(3):
                        uu, vv = u16(b, q + 8 * k + 4), u16(b, q + 8 * k + 6)
                        us.append(uu); vs.append(vv)
                        if want_points:
                            pts.append((mat, uu, vv))
                            w0 = u16(b, q + 8 * k)
                            if not w0 & 0x8000: vpts.append((g, w0, uu, vv))
                    q += 0x18
                    tris += 1
        m = {'verts': nv, 'groups': ng, 'mats': sorted(mats), 'tris': tris,
             'strip_points': strips_pts, 'cloth': ncloth}
        if us:
            m['u'] = [min(us), max(us)]
            m['v'] = [min(vs), max(vs)]
        if want_points:
            m['_pts'] = pts
            m['_vpts'] = vpts
            m['_strips'] = strips
            m['_pos'] = [(i16(b, vp + 6*k), i16(b, vp + 6*k + 2), i16(b, vp + 6*k + 4)) for k in range(nv)]
        models.append(m)
    out['models'] = models
    out['verts'] = sum(m['verts'] for m in models)
    out['tris'] = sum(m['tris'] for m in models)
    return out


if __name__ == '__main__':
    args = sys.argv[1:]
    js = False
    if args and args[0] == '--json':
        js = True
        args = args[1:]
    for p in args:
        try:
            r = parse(p)
        except Exception as ex:  # noqa
            r = {'file': p, 'error': repr(ex)}
        if js:
            print(json.dumps(r))
        else:
            print(json.dumps(r, indent=1))
