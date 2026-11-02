#!/usr/bin/env python3
"""HD texture size vs the PS2 size encoded in the file name.

Names: <tbp>_<psm>_<x>_<y>_<?>_<w>_<h>[_<frame>].dds (maps) or
<tbp>_<psm>_<x>_<y>_<w>_<h>.dds (fp/, effects). Prints a ratio census
per top-level category to stdout; one TSV row per texture to ddsnames.tsv.
"""
import collections
import os
import re
import struct
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'dds/ffx_data/gamedata/ps3data')
PAT = re.compile(r'^(\d+)_(\d+)_(\d+)_(\d+)_(?:(\d+)_)?(\d+)_(\d+)(?:_(\d+))?\.dds$')


def dims(p):
    with open(p, 'rb') as f:
        h = f.read(0x58)
    hh, ww = struct.unpack_from('<II', h, 12)
    fourcc = h[0x54:0x58]
    return ww, hh, fourcc.decode('latin1').strip('\0') or 'RGBA'


def main():
    stats = collections.defaultdict(collections.Counter)
    named = collections.Counter()
    total = collections.Counter()
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'ddsnames.tsv'), 'w') as fo:
        for dp, _, fs in os.walk(ROOT):
            for f in fs:
                if not f.endswith('.dds'):
                    continue
                p = os.path.join(dp, f)
                rel = os.path.relpath(p, ROOT)
                cat = rel.split('/')[0]
                sub = '/'.join(rel.split('/')[:2]) if cat in ('chr', 'map', 'btlmap') else cat
                if cat == 'map' or cat == 'btlmap':
                    kind = 'fp' if '/fp/' in rel else ('2d' if '/2d/' in rel else ('texswap' if 'texswap' in rel else 'main'))
                    sub = cat + '/' + kind
                total[sub] += 1
                w, h, fmt = dims(p)
                m = PAT.match(f)
                if not m:
                    stats[sub]['unnamed %dx%d' % (w, h) if cat in ('menu', 'battle') else 'unnamed'] += 1
                    fo.write(f'{rel}\t{w}\t{h}\t{fmt}\t\t\t\n')
                    continue
                named[sub] += 1
                pw, ph = int(m.group(6)), int(m.group(7))
                if not pw or not ph:
                    stats[sub]["zero-size name"] += 1
                    continue
                rx, ry = w / pw, h / ph
                key = f'x{rx:g}' if rx == ry else f'x{rx:g}/y{ry:g}'
                stats[sub][key] += 1
                fo.write(f'{rel}\t{w}\t{h}\t{fmt}\t{pw}\t{ph}\t{key}\n')
    for sub in sorted(total):
        print(sub, total[sub], 'named', named[sub], dict(stats[sub].most_common(8)))


if __name__ == '__main__':
    main()
