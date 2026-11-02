#!/usr/bin/env python3
"""For each map in maps-sample.txt: PS2 (mapout.vpa) vs HD glb, world space.
pos_hd: fraction of HD vertices (world, x,y negated) on a PS2 vertex (5 cm grid).
pos_ps2: fraction of PS2 unique positions present in HD.
uv_fit: of the matched HD vertices, fraction whose UV is the PS2 UV under one
        power-of-two-scale + offset map per HD primitive.
"""
import collections
import json
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import compare2  # noqa: E402
import mapuv  # noqa: E402
import ps2mapgeom  # noqa: E402

PS2 = os.path.expanduser('~/ffx-work/demaster/ps2/jppc')


def key(a):
    return [tuple(x) for x in np.round(a * 20).astype(np.int64)]


def run(m):
    n = os.path.basename(m)
    vpa = os.path.join(PS2, m, 'bin', 'mapout.vpa')
    glb = os.path.join(HERE, 'hdmap/ffx_data/gamedata/ps3data', m, 'mdl/d3d11', n + '.glb')
    r = {'map': m}
    if not os.path.exists(vpa) or not os.path.exists(glb):
        r['missing'] = [os.path.exists(vpa), os.path.exists(glb)]
        return r
    bt, skipped = ps2mapgeom.batches(vpa)
    import mapcmp
    Pall, _, models = mapcmp.ps2_geom(vpa)
    kp_all = set(key(Pall))
    P = np.concatenate([p for p, u in bt]) if bt else np.zeros((0, 3))
    UV = np.concatenate([u for p, u in bt]) if bt else np.zeros((0, 2))
    d = collections.defaultdict(list)
    for k, uv in zip(key(P), UV):
        d[k].append(uv)
    tot = matched = fit_ok = uv_matched = 0
    kh_all = set()
    tris = 0
    for H, HU in mapuv.prims_world(glb):
        H = H * np.array([-1, -1, 1])
        ks = key(H)
        kh_all.update(ks)
        pairs, idx = [], []
        for i, (k, uv) in enumerate(zip(ks, HU)):
            tot += 1
            if k in kp_all:
                matched += 1
            for c in d.get(k, ()):
                pairs.append((c[0], c[1], uv[0], uv[1]))
                idx.append(i)
        if pairs:
            mask, _ = compare2.fit(np.array(pairs))
            uv_matched += len(set(idx))
            fit_ok += len(set(i for i, f in zip(idx, mask) if f))
    r.update({'ps2_models': models, 'ps2_strip_verts': len(Pall), 'ps2_uniq': len(kp_all),
              'hd_verts': tot, 'hd_uniq': len(kh_all),
              'pos_hd': round(matched / max(1, tot), 3),
              'pos_ps2': round(len(kp_all & kh_all) / max(1, len(kp_all)), 3),
              'uv_fit': round(fit_ok / max(1, uv_matched), 3)})
    return r


if __name__ == '__main__':
    maps = [l.strip() for l in open(os.path.join(HERE, 'maps-sample.txt')) if l.strip()]
    with open(os.path.join(HERE, 'mapsurvey.jsonl'), 'w') as fo:
        for m in maps:
            try:
                r = run(m)
            except Exception as ex:  # noqa
                r = {'map': m, 'error': repr(ex)}
            fo.write(json.dumps(r) + '\n')
            print(json.dumps(r), flush=True)
