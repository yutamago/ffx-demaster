"""Decode FFX (PS2, SLPM_675.13) character and map textures to PNG.

Usage (from anywhere; Pillow via uv):
  uv run --with pillow python -I ps2tex.py chr  <file.chr>  <outdir>
  uv run --with pillow python -I ps2tex.py map  <dir with mapout.vpa/.txa> <outdir>
  uv run --with pillow python -I ps2tex.py info <file>

Formats: see ~/ffx-work/demaster/research/ps2-textures.md.
"""
import json
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gsmem  # noqa: E402

PSM_NAMES = {0x00: "CT32", 0x01: "CT24", 0x02: "CT16", 0x0A: "CT16S", 0x13: "T8", 0x14: "T4",
             0x1B: "T8H", 0x24: "T4HH", 0x2C: "T4HL"}


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def save_indexed(path, w, h, idx, pal):
    from PIL import Image
    im = Image.new("RGBA", (w, h))
    im.putdata([pal[i] for i in idx])
    im.save(path)


def gray_pal(n):
    return [(v, v, v, 255) for v in ((i * 255) // (n - 1) for i in range(n))]


# --------------------------------------------------------------------------
# .chr
# --------------------------------------------------------------------------

def chr_texinfo(b):
    """Return the parsed texture block of a .chr archive or None."""
    if len(b) < 0x20 or u32(b, 4) < 2:
        return None
    base = u32(b, 0x10 + 8 * 1)  # archive entry 1 = texture data
    if base == 0 or base + 0x24 > len(b):
        return None
    t = dict(base=base, nlist=u16(b, base + 4), nclut=u16(b, base + 8), nimg=u16(b, base + 0xC),
             version=u16(b, base + 0x10))
    lp, cp, ip = (u32(b, base + o) for o in (0x14, 0x18, 0x1C))
    t["anim"] = u32(b, base + 0x20) if t["version"] >= 0x1126 else 0
    t["list"] = []
    for i in range(t["nlist"]):
        e = base + lp + 16 * i
        t["list"].append(dict(img=b[e], clut=b[e + 1], psm=u16(b, e + 2), w=u16(b, e + 4),
                              h=u16(b, e + 6), b8=b[e + 8], b9=b[e + 9]))
    t["cluts"] = []
    for i in range(t["nclut"]):
        e = base + cp + 8 * i
        t["cluts"].append(dict(word0=u32(b, e), data=base + u32(b, e + 4)))
    t["imgs"] = []
    for i in range(t["nimg"]):
        e = base + ip + 16 * i
        t["imgs"].append(dict(psm=u16(b, e), blocks=u16(b, e + 2), w=u16(b, e + 8),
                              h=u16(b, e + 0xA), data=base + u32(b, e + 0xC)))
    return t


def chr_clut_raw(b, c):
    n = (u32(b, c["data"] - 0x10) & 0x7FFF)  # GIFtag NLOOP = qwords
    return b[c["data"]:c["data"] + n * 16], n * 4


def chr_img_index(b, im):
    w, h, d = im["w"], im["h"], im["data"]
    if im["psm"] == 0x13:
        return b[d:d + w * h]
    if im["psm"] == 0x14:
        return gsmem.unpack4_linear(b[d:d + w * h // 2], w, h)
    raise ValueError("psm %x" % im["psm"])


def chr_anim(b, t):
    """Texture-animation block (texture data +0x20). Returns patch list."""
    a = t["base"] + t["anim"]
    kind, cnt, ver = b[a], b[a + 1], b[a + 2]
    poff, eoff = u32(b, a + 4), u32(b, a + 8)
    patches = []
    n = (eoff - poff) // 12 if eoff > poff else 0
    for i in range(n):
        e = a + poff + 12 * i
        p = dict(f0=u16(b, e), f2=u16(b, e + 2), w=u16(b, e + 4), h=u16(b, e + 6),
                 hdr=a + u32(b, e + 8))
        p["data"] = p["hdr"] + 0x20
        patches.append(p)
    return dict(off=a, kind=kind, count=cnt, version=ver, patch_off=poff, entry_off=eoff,
                types=list(b[a + 0xC:a + 0x2C]), patches=patches)


def do_chr(path, outdir):
    b = open(path, "rb").read()
    name = os.path.splitext(os.path.basename(path))[0]
    t = chr_texinfo(b)
    os.makedirs(outdir, exist_ok=True)
    if not t:
        print(name, "no textures")
        return
    pals = []
    for c in t["cluts"]:
        raw, n = chr_clut_raw(b, c)
        pals.append(gsmem.palette_rgba(raw, n))
    idxs = [chr_img_index(b, im) for im in t["imgs"]]
    out = []
    for li, e in enumerate(t["list"]):
        im = t["imgs"][e["img"]]
        fn = "%s_t%02d_img%d_clut%d.png" % (name, li, e["img"], e["clut"])
        save_indexed(os.path.join(outdir, fn), im["w"], im["h"], idxs[e["img"]], pals[e["clut"]])
        out.append(fn)
    if t["anim"]:
        an = chr_anim(b, t)
        for pi, p in enumerate(an["patches"]):
            if p["data"] + p["w"] * p["h"] > len(b):
                continue
            idx = b[p["data"]:p["data"] + p["w"] * p["h"]]
            fn = "%s_anim_p%02d_%dx%d.png" % (name, pi, p["w"], p["h"])
            save_indexed(os.path.join(outdir, fn), p["w"], p["h"], idx, pals[0])
            out.append(fn)
    print(name, "->", len(out), "png")


# --------------------------------------------------------------------------
# MAP1 (.vpa / .txa)
# --------------------------------------------------------------------------

def map1_sections(b):
    if b[:4] != b"MAP1":
        return {}
    secs = {}
    for i in range((min(0x80, len(b)) - 0x10) // 4):
        s = u32(b, 0x10 + 4 * i)
        if s and s < len(b):
            secs[i] = s
    return secs


def arc_records(b, s):
    """Records of a yiArc map render file (magic 0x214365, version 0x110)."""
    if u32(b, s) != 0x214365:
        return []
    n, o, recs = u32(b, s + 0xC), s + 0x40, []
    for _ in range(n):
        k, ln = u32(b, o), u32(b, o + 4)
        recs.append(dict(off=o, kind=k, len=ln, hdr=struct.unpack_from("<16I", b, o)))
        o += 0x40 + ln
    return recs


def txa_textures(b):
    texs, cluts = [], None
    for si, s in map1_sections(b).items():
        for r in arc_records(b, s):
            if r["kind"] == 2:
                e = r["off"] + 0x40
                f = struct.unpack_from("<16I", b, e)
                texs.append(dict(rec=r["off"], dbp=f[1], dbw=f[2], psm=f[3], w=f[4], h=f[5],
                                 f14=f[14], data=r["off"] + 0x80, size=r["len"] - 0x40))
            elif r["kind"] == 3:
                d = r["off"] + 0x40
                cluts = dict(mode=r["hdr"][9], data=d, count=r["len"] // 0x400)
    return texs, cluts


CLUT_LAYOUT = {  # mode: (base words, split, page offsets in words) — PS2 0x0018aba8
    2: (0xB4000, 0x20, (0x400, 0x500, 0x600, 0x700)),
    3: (0x18000, 0x20, (0x100, 0x300, 0x500, 0x700)),
    4: (0x82000, 0x10, (0x100, 0x300, 0x500, 0x700)),
}


def clut_cbp_table(mode, count):
    base, split, tab = CLUT_LAYOUT[mode]
    cbp = {}
    for i in range(count):
        if i < split:
            v = (base + 0x800 * (i // 4) + tab[i % 4]) >> 6
        else:
            v = (0xB8000 + 0x100 * (i - split)) >> 6
        cbp[v] = i
    return cbp


def scan_tex0(b):
    """Find GS TEX0_1/TEX0_2 A+D writes (value qword followed by register 6/7)."""
    found = {}
    for o in range(0, len(b) - 16, 8):
        reg = u32(b, o + 8)
        if reg not in (6, 7) or u32(b, o + 12):
            continue
        v = struct.unpack_from("<Q", b, o)[0]
        k = dict(tbp=v & 0x3FFF, tbw=(v >> 14) & 63, psm=(v >> 20) & 63, tw=(v >> 26) & 15,
                 th=(v >> 30) & 15, tcc=(v >> 34) & 1, tfx=(v >> 35) & 3, cbp=(v >> 37) & 0x3FFF,
                 cpsm=(v >> 51) & 15, csm=(v >> 55) & 1, csa=(v >> 56) & 31, cld=(v >> 61) & 7)
        key = (k["tbp"], k["psm"], k["cbp"], k["csa"], k["cpsm"], k["csm"])
        found[key] = found.get(key, 0) + 1
    return found


def map_tex_index(tex, b, mem):
    w, h, d, psm = tex["w"], tex["h"], tex["data"], tex["psm"]
    data = b[d:d + tex["size"]]
    if psm == 0x14:
        # yiMapFileRead 0x0090f900: uploaded as PSMCT32, width/2, height/4, DBW/2
        mem.write32(tex["dbp"], max(1, tex["dbw"] >> 1), w // 2, h // 4, data)
        return mem.read4(tex["dbp"], tex["dbw"], w, h), 16
    if psm == 0x13:
        return data[:w * h], 256  # PSMT8 host transfer = linear
    if psm == 0x1B:
        return data[:w * h], 256  # PSMT8H host transfer = linear
    if psm in (0x24, 0x2C):
        return gsmem.unpack4_linear(data, w, h), 16  # PSMT4HH/HL host transfer = linear
    return None, 0


def ct16_rgba(data, w, h):
    px = []
    for i in range(w * h):
        v = u16(data, 2 * i)
        r, g, bl = (v & 31) << 3, ((v >> 5) & 31) << 3, ((v >> 10) & 31) << 3
        px.append((r | r >> 5, g | g >> 5, bl | bl >> 5, 255 if v & 0x8000 else 0))
    return px


def bg_layers(b, s, end):
    """Background (YNSC) image transfers in .vpa section 0: BITBLTBUF hi word,
    TRXPOS, TRXREG, then a GIF IMAGE tag (rcbgSceneLoad)."""
    out = []
    o = s
    while o < end - 0x30:
        lo = struct.unpack_from("<Q", b, o)[0]
        nloop, flg = lo & 0x7FFF, (lo >> 58) & 3
        if flg == 2 and nloop and (lo >> 15) & 1 and u32(b, o + 8) == 0 and u32(b, o + 12) == 0:
            bb = struct.unpack_from("<Q", b, o - 0x20)[0]
            w, h = u32(b, o - 0x10), u32(b, o - 0xC)
            dbp, dbw, dpsm = (bb >> 32) & 0x3FFF, (bb >> 48) & 63, (bb >> 56) & 63
            if 0 < w <= 1024 and 0 < h <= 1024 and dpsm in PSM_NAMES:
                out.append(dict(tag=o, dbp=dbp, dbw=dbw, psm=dpsm, w=w, h=h, data=o + 16,
                                size=nloop * 16))
                o += 16 + nloop * 16
                continue
        o += 16
    return out


def do_map(mapdir, outdir):
    vpa = open(os.path.join(mapdir, "mapout.vpa"), "rb").read()
    txa = open(os.path.join(mapdir, "mapout.txa"), "rb").read()
    name = os.path.basename(os.path.dirname(os.path.normpath(mapdir)))
    os.makedirs(outdir, exist_ok=True)
    texs, cl = txa_textures(txa)
    uses = scan_tex0(vpa)
    craw = [txa[cl["data"] + 0x400 * i:cl["data"] + 0x400 * (i + 1)] for i in range(cl["count"])] if cl else []
    # CLUT mode: the record's (kind 0 keeps the previous one — pick the mode resolving most CBPs)
    modes = [cl["mode"]] if cl and cl["mode"] in CLUT_LAYOUT else [4, 2, 3]
    best = max(modes, key=lambda m: sum(1 for k in uses if k[2] in clut_cbp_table(m, len(craw))))
    cbpmap = clut_cbp_table(best, len(craw)) if craw else {}
    mem = gsmem.GSMem()
    manifest = dict(map=name, clut_mode=cl and cl["mode"], clut_layout_used=best, textures=[])
    nout = 0
    for ti, t in enumerate(texs):
        idx, ncol = map_tex_index(t, txa, mem)
        rec = dict(index=ti, dbp=hex(t["dbp"]), dbw=t["dbw"], psm=PSM_NAMES.get(t["psm"], hex(t["psm"])),
                   w=t["w"], h=t["h"], txa_offset=hex(t["data"]), files=[])
        stem = "%s_tex%02d_%04x_%s" % (name, ti, t["dbp"], rec["psm"])
        if t["psm"] == 0x02:
            from PIL import Image
            im = Image.new("RGBA", (t["w"], t["h"]))
            im.putdata(ct16_rgba(txa[t["data"]:t["data"] + t["size"]], t["w"], t["h"]))
            fn = stem + ".png"
            im.save(os.path.join(outdir, fn))
            rec["files"].append(fn)
        elif idx is not None:
            combos = sorted(k for k in uses if k[0] == t["dbp"] and k[1] == t["psm"])
            for (_, _, cbp, csa, cpsm, csm) in combos:
                ci = cbpmap.get(cbp)
                if ci is None:
                    continue
                if ncol == 16:
                    pal = gsmem.palette16_from_256(craw[ci], csa)
                else:
                    pal = gsmem.palette_rgba(craw[ci], 256)
                fn = "%s_clut%02d_csa%02d.png" % (stem, ci, csa)
                save_indexed(os.path.join(outdir, fn), t["w"], t["h"], idx, pal)
                rec["files"].append(fn)
            if not rec["files"]:
                # not referenced by a static TEX0 in the .vpa: CLUT unknown
                pal = gray_pal(ncol)
                fn = stem + "_nopal_gray.png"
                save_indexed(os.path.join(outdir, fn), t["w"], t["h"], idx, pal)
                rec["files"].append(fn)
        nout += len(rec["files"])
        manifest["textures"].append(rec)
    # 2D background layers in .vpa section 0
    secs = map1_sections(vpa)
    if 0 in secs:
        nxt = min([v for v in secs.values() if v > secs[0]] + [len(vpa)])
        lays = bg_layers(vpa, secs[0], nxt)
        imgs = [l for l in lays if l["psm"] in (0x13, 0x14)]
        cls = [l for l in lays if l["psm"] == 0 and l["w"] == 16 and l["h"] == 16]
        for li, l in enumerate(imgs):
            if l["psm"] == 0x13:
                idx = vpa[l["data"]:l["data"] + l["w"] * l["h"]]
                pal = gsmem.palette_rgba(vpa[cls[0]["data"]:cls[0]["data"] + 0x400], 256) if cls else gray_pal(256)
            else:
                idx = gsmem.unpack4_linear(vpa[l["data"]:], l["w"], l["h"])
                pal = gray_pal(16)
            fn = "%s_bg%02d_%04x_%s_%dx%d.png" % (name, li, l["dbp"], PSM_NAMES[l["psm"]], l["w"], l["h"])
            save_indexed(os.path.join(outdir, fn), l["w"], l["h"], idx, pal)
            manifest.setdefault("bg_layers", []).append(dict(file=fn, dbp=hex(l["dbp"]), vpa_offset=hex(l["data"]),
                                                             clut_offset=hex(cls[0]["data"]) if cls else None))
            nout += 1
    with open(os.path.join(outdir, name + "_manifest.json"), "w") as f:
        json.dump(manifest, f, indent=1)
    print(name, "->", nout, "png (clut layout mode %s)" % best)


def do_info(path):
    b = open(path, "rb").read()
    if b[:4] == b"MAP1":
        texs, cl = txa_textures(b)
        for t in texs:
            print("tex dbp=%04x dbw=%d psm=%s %dx%d data=%x" % (t["dbp"], t["dbw"], PSM_NAMES.get(t["psm"]),
                                                              t["w"], t["h"], t["data"]))
        print("cluts", cl)
        return
    t = chr_texinfo(b)
    print(json.dumps({k: v for k, v in t.items()}, indent=1, default=hex) if t else "no textures")


if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "chr":
        do_chr(sys.argv[2], sys.argv[3])
    elif cmd == "map":
        do_map(sys.argv[2], sys.argv[3])
    elif cmd == "info":
        do_info(sys.argv[2])
