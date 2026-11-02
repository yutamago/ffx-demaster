#!/usr/bin/env python3
"""Geometry + UV correspondence between HD chr models and PS2 .chr.

pos_hd: fraction of HD unique vertex positions (x1000) that are PS2 vertices.
pos_ps2: fraction of PS2 vertices present in HD.
uv_fit: fraction of HD (position, uv) pairs explained by one axis-aligned
        affine map (power-of-two scale, any offset) per PS2 strip group.
uv_id: same, with the scale fixed to identity in normalised PS2 page units
       (u/4096) and offset 0 (i.e. the HD UVs are the PS2 UVs unchanged).
Writes compare2.jsonl.
"""
import collections
import glob
import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import glbstat  # noqa: E402
import ps2chr  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
PS2 = os.path.expanduser('~/ffx-work/demaster/ps2/jppc/chr')
SCALES = [2.0 ** k for k in range(-3, 4)]
TOL = 1.0 / 512


def edge_hist(segs):
    c = collections.Counter()
    for L in segs:
        c[int(round(L * 200))] += 1  # 5 mm bins (HD units)
    return c


def ps2_edges(d):
    out = []
    for m in d.get('models', []):
        P = np.array(m['_pos'], float) / 1000
        es = set()
        for s in m.get('_strips', []):
            for i in range(len(s) - 2):
                a, b, c = s[i], s[i + 1], s[i + 2]
                for x, y in ((a, b), (b, c), (a, c)):
                    if x != y and x < len(P) and y < len(P):
                        es.add((min(x, y), max(x, y)))
        out.extend(float(np.linalg.norm(P[x] - P[y])) for x, y in es)
    return out


def hd_edges(path):
    js, binc = glbstat.load(path)
    out = []
    for m in js.get('meshes', []):
        for p in m['primitives']:
            pos = np.array(glbstat.accessor(js, binc, p['attributes']['POSITION'])[0])
            key = {}
            ids = np.array([key.setdefault(tuple(np.round(q, 4)), len(key)) for q in pos])
            if 'indices' in p:
                ix = np.array(glbstat.accessor(js, binc, p['indices'])[0]).reshape(-1)
            else:
                ix = np.arange(len(pos))
            uniq_pos = {}
            for q, i in zip(pos, ids):
                uniq_pos[i] = q
            es = set()
            t = ids[ix].reshape(-1, 3)
            for a, b, c in t:
                for x, y in ((a, b), (b, c), (a, c)):
                    if x != y:
                        es.add((min(x, y), max(x, y)))
            out.extend(float(np.linalg.norm(uniq_pos[x] - uniq_pos[y])) for x, y in es)
    return out


def hd_pairs(path):
    js, binc = glbstat.load(path)
    out = []
    for m in js.get('meshes', []):
        for p in m['primitives']:
            at = p['attributes']
            pos, _ = glbstat.accessor(js, binc, at['POSITION'])
            uv = glbstat.accessor(js, binc, at['TEXCOORD_0'])[0] if 'TEXCOORD_0' in at else [(0, 0)] * len(pos)
            for q, t in zip(pos, uv):
                out.append((tuple(int(round(c * 1000)) for c in q[:3]), (t[0], t[1])))
    return out


def snap(q, uniq):
    q = tuple(int(c) for c in q)
    if q in uniq:
        return q
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            for dz in (-1, 0, 1):
                t = (q[0] + dx, q[1] + dy, q[2] + dz)
                if t in uniq:
                    return t
    return q


def best_transform(P, H, uniq):
    """Signed axis permutation + translation taking PS2 positions to HD ones."""
    import itertools
    best = (-1, None, None)
    sample = P[:: max(1, len(P) // 400)]
    for perm in itertools.permutations(range(3)):
        for signs in itertools.product((1, -1), repeat=3):
            Q = P[:, perm] * np.array(signs)
            for anchor in ('id', 'min', 'max', 'mid'):
                if anchor == 'id':
                    t = np.zeros(3)
                elif anchor == 'min':
                    t = H.min(0) - Q.min(0)
                elif anchor == 'max':
                    t = H.max(0) - Q.max(0)
                else:
                    t = (H.min(0) + H.max(0)) / 2 - (Q.min(0) + Q.max(0)) / 2
                t = np.round(t)
                S = sample[:, perm] * np.array(signs) + t
                hit = sum(snap(q, uniq) in uniq for q in S)
                if hit > best[0]:
                    best = (hit, (perm, signs, anchor), t)
                if anchor == 'id' and hit == len(S):
                    break
    perm, signs, anchor = best[1]
    Q = np.round(P[:, perm] * np.array(signs) + best[2])
    desc = 'id' if (perm == (0, 1, 2) and signs == (1, 1, 1) and not best[2].any()) else \
        f'perm={perm} sign={signs} t={best[2].astype(int).tolist()}'
    return Q, desc


def fit(pairs):
    """pairs: array (n, 4) of ps2 u, v (normalised) and hd u, v.
    Returns (inlier mask, (sx, ox, sy, oy))."""
    best = (-1, None, None)
    for sx in SCALES:
        offx = pairs[:, 2] - sx * pairs[:, 0]
        bins = np.round(offx * 512).astype(int)
        vals, cnt = np.unique(bins, return_counts=True)
        ox = vals[cnt.argmax()] / 512
        mx = np.abs(offx - ox) <= TOL
        for sy in SCALES + [-s for s in SCALES]:
            offy = pairs[:, 3] - sy * pairs[:, 1]
            bins = np.round(offy[mx] * 512).astype(int) if mx.any() else np.array([0])
            vals, cnt = np.unique(bins, return_counts=True)
            oy = vals[cnt.argmax()] / 512
            m = mx & (np.abs(offy - oy) <= TOL)
            if m.sum() > best[0]:
                best = (m.sum(), m, (sx, ox, sy, oy))
    return best[1], best[2]


def analyse(kind, name, glb):
    p = os.path.join(PS2, kind, name, 'mdl', name + '.chr')
    r = {'kind': kind, 'name': name}
    if not os.path.exists(p):
        r['ps2_missing'] = True
        return r
    d = ps2chr.parse(p, want_points=True)
    hp = hd_pairs(glb)
    uniq = set(q for q, _ in hp)
    allp = [(mi, vi, q) for mi, m in enumerate(d.get('models', [])) for vi, q in enumerate(m['_pos'])]
    if not uniq or not allp:
        r['empty'] = True
        return r
    xf = best_transform(np.array([q for _, _, q in allp]), np.array(sorted(uniq)), uniq)
    r['xform'] = xf[1]
    pos_index = collections.defaultdict(list)
    pts_by_vertex = collections.defaultdict(list)
    for (mi, vi, _), q in zip(allp, xf[0]):
        pos_index[snap(q, uniq)].append((mi, vi))
    for mi, m in enumerate(d.get('models', [])):
        for g, vi, u, v in m['_vpts']:
            pts_by_vertex[(mi, vi)].append(((mi, g), u / 4096, v / 4096))
    eh, ep = edge_hist(hd_edges(glb)), edge_hist(ps2_edges(d))
    inter = sum((eh & ep).values())
    r['edges_hd'], r['edges_ps2'] = sum(eh.values()), sum(ep.values())
    r['edge_overlap'] = round(inter / max(1, max(r['edges_hd'], r['edges_ps2'])), 3)
    r['pos_hd'] = round(sum(q in pos_index for q in uniq) / len(uniq), 3)
    r['pos_ps2'] = round(sum(q in uniq for q in pos_index) / len(pos_index), 3)
    # UV pairs per PS2 group
    hd_uv = list(set((q, (round(t[0], 5), round(t[1], 5))) for q, t in hp))
    groups = collections.defaultdict(list)  # key -> list of (hd idx, ps2u, ps2v, hdu, hdv)
    matched = 0
    for k, (q, t) in enumerate(hd_uv):
        if q not in pos_index:
            continue
        matched += 1
        for mv in pos_index[q]:
            for key, u, v in pts_by_vertex.get(mv, ()):
                groups[key].append((k, u, v, t[0], t[1]))
    if not matched:
        r['uv_fit'] = 0.0
        return r
    ok_fit = set()
    ok_id = set()
    maps = collections.Counter()
    for key, lst in groups.items():
        a = np.array([x[1:] for x in lst])
        idx = [x[0] for x in lst]
        m, prm = fit(a)
        if m is not None:
            ok_fit.update(i for i, f in zip(idx, m) if f)
            maps[prm] += int(m.sum())
        for flip in (False, True):
            hv = 1 - a[:, 3] if flip else a[:, 3]
            mid = (np.abs(a[:, 2] - a[:, 0]) <= TOL) & (np.abs(hv - a[:, 1]) <= TOL)
            ok_id.update(i for i, f in zip(idx, mid) if f)
    r['uv_matched_pairs'] = matched
    r['uv_fit'] = round(len(ok_fit) / matched, 3)
    r['uv_id'] = round(len(ok_id) / matched, 3)
    r['uv_maps'] = [list(k) + [v] for k, v in maps.most_common(6)]
    r['uv_map_count'] = len(maps)
    return r


def main():
    files = sorted(glob.glob(os.path.join(HERE, 'hd/ffx_data/gamedata/ps3data/chr/*/*/mdl/d3d11/*.glb')))
    if len(sys.argv) > 1:
        files = [f for f in files if any('/' + a + '/' in f for a in sys.argv[1:])]
    out = os.path.join(HERE, 'compare2.jsonl' if len(sys.argv) == 1 else 'compare2-sel.jsonl')
    with open(out, 'w') as fo:
        for f in files:
            parts = f.split('/')
            kind, name = parts[-5], parts[-4]
            try:
                r = analyse(kind, name, f)
            except Exception as ex:  # noqa
                r = {'kind': kind, 'name': name, 'error': repr(ex)}
            fo.write(json.dumps(r) + '\n')
            print(kind, name, r.get('edge_overlap'), r.get('pos_hd'), r.get('pos_ps2'), r.get('uv_fit'), r.get('uv_id'),
                  r.get('uv_map_count'), (r.get('uv_maps') or [None])[0], sep='\t', flush=True)


if __name__ == '__main__':
    main()
