"""Check that the bytes zeroed in the PC copies of PS2 .chr files are exactly the
texture pixel data located by ps2tex.chr_texinfo (images + texture-animation patches).

  python -I verify_pc_zeroed.py <ps2 root> <pc root>
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ps2tex  # noqa: E402


def ranges_of_diff(a, b):
    out, s = [], None
    for i in range(min(len(a), len(b))):
        d = a[i] != b[i]
        if d and s is None:
            s = i
        elif not d and s is not None:
            out.append((s, i))
            s = None
    if s is not None:
        out.append((s, min(len(a), len(b))))
    return out


def main(ps2root, pcroot):
    stats = dict(files=0, same_size=0, all_inside=0, outside=0, pc_nonzero_inside=0, untouched_tex=0)
    bad = []
    for dp, _, fs in os.walk(os.path.join(ps2root, "jppc", "chr")):
        for f in fs:
            if not f.endswith(".chr"):
                continue
            p = os.path.join(dp, f)
            rel = os.path.relpath(p, ps2root)
            q = os.path.join(pcroot, rel)
            if not os.path.exists(q):
                continue
            a, b = open(p, "rb").read(), open(q, "rb").read()
            stats["files"] += 1
            if len(a) != len(b):
                continue
            stats["same_size"] += 1
            t = ps2tex.chr_texinfo(a)
            regs = []
            if t:
                for im in t["imgs"]:
                    n = im["w"] * im["h"] * (1 if im["psm"] == 0x13 else 0.5)
                    regs.append((im["data"], im["data"] + int(n)))
                if t["anim"]:
                    for pt in ps2tex.chr_anim(a, t)["patches"]:
                        regs.append((pt["data"], pt["data"] + pt["w"] * pt["h"]))
            diffs = ranges_of_diff(a, b)
            ok = all(any(r0 <= s and e <= r1 for r0, r1 in regs) for s, e in diffs)
            # PC side of every texture region must be all zero
            pcz = all(not any(b[r0:r1]) for r0, r1 in regs)
            if ok:
                stats["all_inside"] += 1
            else:
                stats["outside"] += 1
                bad.append((rel, [(hex(s), hex(e)) for s, e in diffs if not any(r0 <= s and e <= r1 for r0, r1 in regs)][:4]))
            if not pcz:
                stats["untouched_tex"] += 1
    print(stats)
    for x in bad[:20]:
        print("outside:", x)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
