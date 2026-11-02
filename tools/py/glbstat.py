#!/usr/bin/env python3
"""Stats of a .glb made by `ffx model export`: verts, tris, unique positions,
UV ranges, textures (PNG sizes), material sampler imports.
Usage: glbstat.py [--json] file.glb...
"""
import json
import struct
import sys

CT = {5120: 'b', 5121: 'B', 5122: 'h', 5123: 'H', 5125: 'I', 5126: 'f'}
NC = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}


def load(path):
    b = open(path, 'rb').read()
    assert b[:4] == b'glTF'
    off = 12
    js, binc = None, None
    while off < len(b):
        ln, ty = struct.unpack_from('<II', b, off)
        c = b[off + 8: off + 8 + ln]
        if ty == 0x4E4F534A:
            js = json.loads(c)
        elif ty == 0x004E4942:
            binc = c
        off += 8 + ln
    return js, binc


def accessor(js, binc, i):
    a = js['accessors'][i]
    bv = js['bufferViews'][a['bufferView']]
    n = NC[a['type']]
    fmt = CT[a['componentType']]
    sz = struct.calcsize(fmt)
    stride = bv.get('byteStride', sz * n)
    base = bv.get('byteOffset', 0) + a.get('byteOffset', 0)
    out = []
    for k in range(a['count']):
        out.append(struct.unpack_from('<' + fmt * n, binc, base + k * stride))
    return out, a


def png_size(data):
    if data[:8] == b'\x89PNG\r\n\x1a\n':
        return struct.unpack('>II', data[16:24])
    return None


def stat(path):
    js, binc = load(path)
    r = {'file': path, 'meshes': len(js.get('meshes', [])), 'prims': 0, 'verts': 0,
         'tris': 0, 'skins': len(js.get('skins', [])),
         'joints_max': max([len(s['joints']) for s in js.get('skins', [])] or [0])}
    pos_all = set()
    umin = vmin = 1e9
    umax = vmax = -1e9
    prims = []
    for mi, m in enumerate(js.get('meshes', [])):
        for p in m['primitives']:
            at = p['attributes']
            pos, a = accessor(js, binc, at['POSITION'])
            nv = len(pos)
            if 'indices' in p:
                nt = js['accessors'][p['indices']]['count'] // 3
            else:
                nt = nv // 3
            up = set((round(x, 4), round(y, 4), round(z, 4)) for x, y, z in pos)
            pos_all |= set((mi,) + q for q in up)
            pr = {'mesh': mi, 'verts': nv, 'tris': nt, 'uniq_pos': len(up),
                  'material': p.get('material')}
            if 'TEXCOORD_0' in at:
                uv, _ = accessor(js, binc, at['TEXCOORD_0'])
                us = [q[0] for q in uv]
                vs = [q[1] for q in uv]
                pr['u'] = [round(min(us), 3), round(max(us), 3)]
                pr['v'] = [round(min(vs), 3), round(max(vs), 3)]
                umin, umax = min(umin, min(us)), max(umax, max(us))
                vmin, vmax = min(vmin, min(vs)), max(vmax, max(vs))
            prims.append(pr)
            r['prims'] += 1
            r['verts'] += nv
            r['tris'] += nt
    r['uniq_pos'] = len(pos_all)
    if umin < 1e9:
        r['uv'] = [round(umin, 3), round(umax, 3), round(vmin, 3), round(vmax, 3)]
    imgs = []
    for im in js.get('images', []):
        sz = None
        if 'bufferView' in im:
            bv = js['bufferViews'][im['bufferView']]
            o = bv.get('byteOffset', 0)
            sz = png_size(binc[o:o + 32])
        imgs.append({'name': im.get('name'), 'size': sz})
    r['images'] = imgs
    texs = set()
    for mat in js.get('materials', []):
        ex = mat.get('extras', {})
        t = ex.get('ffx_textures') or {}
        if isinstance(t, dict):
            for k, v in t.items():
                texs.add(f'{k}={v}')
    r['texture_imports'] = sorted(texs)
    r['materials'] = len(js.get('materials', []))
    r['prim_list'] = prims
    return r


if __name__ == '__main__':
    args = sys.argv[1:]
    js = args and args[0] == '--json'
    if js:
        args = args[1:]
    for p in args:
        r = stat(p)
        if js:
            print(json.dumps(r))
        else:
            pl = r.pop('prim_list')
            print(json.dumps(r, indent=1))
            for q in pl:
                print('  ', q)
