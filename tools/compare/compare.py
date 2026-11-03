"""PS2 (PCSX2) vs Demaster (the PC game with the mod) side by side
(docs/compare.md).

    uv run --with pillow --with numpy python -I tools/compare/compare.py <command> [replay...]

Commands (replays default to every [[screen]] of scenes.toml):

  ps2   play each screen's PS2 script in PCSX2 (software renderer, native)
        from its save state; every `shot` keeps the PS2 frame (PCSX2's
        screenshot, 512x416) and a single-frame GS dump of it in
        <out>/ps2/<replay>/. Starts PCSX2 through pcsx2.sh when none runs,
        and stops it after.
  hw    replay each GS dump with the hardware renderer (--hw vk|dx12|dx11,
        default vk) at --hw-scale (default 2: 1024x832, the size above the
        Demaster's 720 lines) into <out>/ps2hw/<replay>/; the ini goes back to
        the software renderer at native resolution afterwards.
  pc    play each screen's PC replay through tools/calllist (ffx-rust
        harness) with the converter's output as the mod, in German
        (FFX_LANG=de for ffx-rust's run-game.sh; the replay's menu_us paths
        -> menu_de) so the text matches the German PAL disc; shots in
        <out>/pc/<replay>/.
  diff  two PNGs per pair, PS2 | Demaster | difference:
          <replay>-<shot>-exact.png   the software renderer's native frame
              (512x416) against the Demaster's 4:3 part scaled down to it:
              colours, blending, layout;
          <replay>-<shot>-detail.png  the hardware renderer's upscaled frame
              scaled to the Demaster's 4:3 part (960x720) against the
              Demaster at full resolution: textures, edges, small text.
        Per-region numbers (both) in <out>/report.tsv and on stdout.
  all   ps2, hw, pc, then diff (one game at a time).

Options: --out <dir> (default /mnt/c/shots/compare), --mod <efl/x dir>
(default ~/ffx-work/demaster/scratch/compare-ui/efl/x), --lang de|en (PC
language, default de), --hw vk|dx12|dx11, --hw-scale N.
"""
import glob
import os
import subprocess
import sys
import time
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
PCSX2_DIR = os.environ.get('PCSX2', '/mnt/c/_workspaces/pcsx2')
# the PS2 frame (PAL menus, battle, title: 512x416) and the Demaster's 4:3
# part of its 1280x720 window
PS2_W, PS2_H = 512, 416
PC_W, PC_H = 960, 720


def find_decomp():
    """The ffx-rust checkout next to the repository (or next to the main
    checkout, from a git worktree under .claude/worktrees)."""
    d = REPO
    while d != os.path.dirname(d):
        c = os.path.join(os.path.dirname(d), 'ffx-decomp')
        if os.path.isdir(os.path.join(c, 'crates', 'ffx-harness')):
            return c
        d = os.path.dirname(d)
    sys.exit('no ffx-decomp checkout found next to the repository')


def winpath(p):
    return subprocess.run(['wslpath', '-w', p], capture_output=True, text=True, check=True).stdout.strip()


def load_screens(names):
    with open(os.path.join(HERE, 'scenes.toml'), 'rb') as f:
        screens = tomllib.load(f)['screen']
    if names:
        screens = [s for s in screens if s['replay'] in names]
        missing = set(names) - {s['replay'] for s in screens}
        if missing:
            sys.exit('not in scenes.toml: ' + ', '.join(sorted(missing)))
    return screens


# ------------------------------------------------------------------ PS2

def pcsx2(*args, check=True, env=None):
    return subprocess.run([os.path.join(HERE, 'pcsx2.sh'), *args], check=check, env=env)


def pcsx2_running():
    out = subprocess.run(['tasklist.exe', '/FI', 'IMAGENAME eq pcsx2-qt.exe', '/FO', 'CSV', '/NH'],
                         capture_output=True, text=True).stdout
    return 'pcsx2-qt' in out


def set_renderer(kind, scale):
    subprocess.run([sys.executable, '-I', os.path.join(HERE, 'setup.py'), 'renderer', PCSX2_DIR,
                    kind, str(scale)], check=True)


def clean_snaps():
    """The GS dump hotkey also writes a PNG preview next to the dump."""
    for p in glob.glob(os.path.join(PCSX2_DIR, 'snaps', '*.png')):
        os.remove(p)


def with_dumps(script):
    """Each `shot $OUT\\X.png` line followed by `gsdump $OUT\\X.gs.zst`."""
    out = []
    for line in script.splitlines():
        out.append(line)
        w = line.split()
        if len(w) == 2 and w[0] == 'shot' and w[1].endswith('.png'):
            out.append('gsdump ' + w[1][:-4] + '.gs.zst')
    return '\n'.join(out) + '\n'


def run_ps2(screens, out):
    started = False
    if not pcsx2_running():
        set_renderer('sw', 1)
        pcsx2('start')
        started = True
    try:
        sdir = os.path.join(out, 'scripts')
        os.makedirs(sdir, exist_ok=True)
        for s in screens:
            od = os.path.join(out, 'ps2', s['replay'])
            os.makedirs(od, exist_ok=True)
            script = f"state {s['state']}\nwait 30\n" + with_dumps(s['ps2'])
            sf = os.path.join(sdir, s['replay'] + '.txt')
            with open(sf, 'w') as f:
                f.write(script)
            print(f"== PS2 {s['replay']}", flush=True)
            pcsx2('ctl', 'run', winpath(sf), 'OUT=' + winpath(od))
    finally:
        if started:
            pcsx2('stop', check=False)
        clean_snaps()


def run_hw(screens, out, kind, scale):
    if pcsx2_running():
        sys.exit('PCSX2 is running: stop it first (one game at a time)')
    set_renderer(kind, scale)
    try:
        for s in screens:
            for name in s['pairs']:
                dump = os.path.join(out, 'ps2', s['replay'], name + '.gs.zst')
                if not os.path.exists(dump):
                    print(f"{s['replay']} {name}: no GS dump (run ps2 first)")
                    continue
                od = os.path.join(out, 'ps2hw', s['replay'])
                os.makedirs(od, exist_ok=True)
                print(f"== HW {kind} x{scale} {s['replay']} {name}", flush=True)
                pcsx2('start', env=dict(os.environ, PCSX2_ISO=winpath(dump)))
                try:
                    time.sleep(3)
                    pcsx2('ctl', 'shot', winpath(os.path.join(od, name + '.png')))
                finally:
                    pcsx2('stop', check=False)
    finally:
        set_renderer('sw', 1)
        clean_snaps()


# ------------------------------------------------------------------ PC

def calllist_manifest():
    """tools/calllist's manifest; from a worktree, where its relative path
    to ffx-harness does not resolve, a copy in scratch with the absolute one."""
    src = os.path.join(REPO, 'tools', 'calllist')
    if os.path.isdir(os.path.join(src, '..', '..', '..', 'ffx-decomp', 'crates')):
        return os.path.join(src, 'Cargo.toml')
    import shutil
    dst = os.path.expanduser('~/ffx-work/demaster/scratch/calllist-build')
    os.makedirs(dst, exist_ok=True)
    for name in ('src', 'thunk.s', 'Cargo.lock'):
        a, b = os.path.join(src, name), os.path.join(dst, name)
        if os.path.isdir(a):
            shutil.copytree(a, b, dirs_exist_ok=True)
        elif os.path.exists(a):
            shutil.copy(a, b)
    toml = open(os.path.join(src, 'Cargo.toml')).read()
    toml = toml.replace('../../../ffx-decomp', find_decomp())
    with open(os.path.join(dst, 'Cargo.toml'), 'w') as f:
        f.write(toml)
    # max.ini is found through CARGO_MANIFEST_DIR/../sandbox
    os.makedirs(os.path.join(os.path.dirname(dst), 'sandbox'), exist_ok=True)
    shutil.copy(os.path.join(REPO, 'tools', 'sandbox', 'max.ini'), os.path.join(os.path.dirname(dst), 'sandbox', 'max.ini'))
    return os.path.join(dst, 'Cargo.toml')


def run_pc(screens, out, mod, lang):
    if pcsx2_running():
        sys.exit('PCSX2 is running: stop it first (one game at a time)')
    from PIL import Image
    rdir = os.path.join(out, 'replays')
    os.makedirs(rdir, exist_ok=True)
    us = 'menu_us'.encode().hex()
    for s in screens:
        src = open(os.path.join(REPO, 'tools', 'replays', s['replay'] + '.txt')).read()
        if lang != 'en':
            src = src.replace(us, f'menu_{lang}'.encode().hex())
        rf = os.path.join(rdir, s['replay'] + '.txt')
        with open(rf, 'w') as f:
            f.write(src)
        od = os.path.join(out, 'pc', s['replay'])
        os.makedirs(od, exist_ok=True)
        print(f"== PC {s['replay']}", flush=True)
        env = dict(os.environ, RUSTC_WRAPPER='', FFX_LANG=lang)
        env.setdefault('FFX_GAME_DIR', os.path.expanduser('~/steam-games/FFX-X2-HD'))
        subprocess.run(['cargo', 'run', '-j', '4', '--release', '--manifest-path',
                        calllist_manifest(), '--',
                        rf, winpath(od) + '\\trace', mod], cwd=find_decomp(), env=env, check=True)
        # the harness writes 3.6 MB BMPs: keep PNGs
        for name in sorted(os.listdir(od)):
            if name.endswith('.bmp'):
                b = os.path.join(od, name)
                Image.open(b).convert('RGB').save(b[:-4] + '.png')
                os.remove(b)


# ------------------------------------------------------------------ diff

def pc43(img):
    """The Demaster's 4:3 part of its 16:9 window (render.aspect
    pillarboxes it), at full resolution."""
    w, h = img.size
    if abs(w / h - 16 / 9) < 0.01:
        w43 = round(h * 4 / 3)
        x0 = (w - w43) // 2
        img = img.crop((x0, 0, x0 + w43, h))
    return img


def find_shot(d, name):
    for ext in ('.png', '.bmp'):
        p = os.path.join(d, name + ext)
        if os.path.exists(p):
            return p
    return None


def stats(d):
    """Mean absolute difference (0..255, mean of RGB) and the share of
    pixels whose largest channel difference is over 32."""
    m = d.max(axis=2)
    return float(d.mean()), float((m > 32).mean() * 100)


def panel(a, b, title_a, title_b, regions, scale):
    """PS2 | Demaster | difference with region boxes; returns the image and
    the numbers (whole frame first)."""
    import numpy as np
    from PIL import Image, ImageDraw
    w, h = a.size
    A = np.asarray(a).astype(np.int16)
    B = np.asarray(b).astype(np.int16)
    D = np.abs(A - B)
    heat = np.clip(D.max(axis=2) * 2, 0, 255).astype(np.uint8)
    dimg = Image.merge('RGB', [Image.fromarray(heat), Image.fromarray(heat // 2),
                               Image.fromarray(np.zeros_like(heat))])
    canvas = Image.new('RGB', (w * 3, h + 20), (24, 24, 24))
    canvas.paste(a, (0, 20))
    canvas.paste(b, (w, 20))
    canvas.paste(dimg, (2 * w, 20))
    dr = ImageDraw.Draw(canvas)
    whole = stats(D)
    dr.text((6, 4), title_a, fill=(255, 255, 255))
    dr.text((w + 6, 4), title_b, fill=(255, 255, 255))
    dr.text((2 * w + 6, 4), f'difference  mean {whole[0]:.1f}  >32: {whole[1]:.0f}%', fill=(255, 255, 255))
    rows = [('all', *whole)]
    for rname, (x, y, rw, rh) in regions.items():
        x0, y0 = round(x * scale[0]), round(y * scale[1])
        x1, y1 = round((x + rw) * scale[0]), round((y + rh) * scale[1])
        st = stats(D[y0:y1, x0:x1])
        rows.append((rname, *st))
        dr.rectangle((2 * w + x0, 20 + y0, 2 * w + x1 - 1, 20 + y1 - 1), outline=(80, 160, 255))
        dr.text((2 * w + x0 + 3, 20 + y0 + 2), f'{rname} {st[0]:.0f}', fill=(120, 190, 255))
    return canvas, rows


def run_diff(screens, out):
    from PIL import Image
    rows = []
    for s in screens:
        regions = s.get('regions', {})
        for ps2name, pcname in s['pairs'].items():
            q = find_shot(os.path.join(out, 'pc', s['replay']), pcname)
            if not q:
                print(f"{s['replay']} {ps2name}: no PC shot {pcname}")
                continue
            pc = pc43(Image.open(q).convert('RGB'))
            # exact: software renderer, native
            p = find_shot(os.path.join(out, 'ps2', s['replay']), ps2name)
            if p:
                a = Image.open(p).convert('RGB')
                b = pc.resize(a.size, Image.Resampling.BOX)
                sx, sy = a.size[0] / PS2_W, a.size[1] / PS2_H
                img, r = panel(a, b, f'PS2 (software, native {a.size[0]}x{a.size[1]})  {ps2name}',
                               f'Demaster {pcname}, scaled down', regions, (sx, sy))
                png = os.path.join(out, f"{s['replay']}-{ps2name}-exact.png")
                img.save(png)
                print('wrote', png)
                rows += [(s['replay'], ps2name, 'exact', *x) for x in r]
            # detail: hardware renderer, upscaled
            p = find_shot(os.path.join(out, 'ps2hw', s['replay']), ps2name)
            if p:
                hw = Image.open(p).convert('RGB')
                a = hw.resize(pc.size, Image.Resampling.LANCZOS)
                sx, sy = pc.size[0] / PS2_W, pc.size[1] / PS2_H
                img, r = panel(a, pc, f'PS2 (hardware {hw.size[0]}x{hw.size[1]} -> {pc.size[0]}x{pc.size[1]})  {ps2name}',
                               f'Demaster {pcname}, full resolution', regions, (sx, sy))
                png = os.path.join(out, f"{s['replay']}-{ps2name}-detail.png")
                img.save(png)
                print('wrote', png)
                rows += [(s['replay'], ps2name, 'detail', *x) for x in r]
    report = os.path.join(out, 'report.tsv')
    with open(report, 'w') as f:
        f.write('replay\tshot\tkind\tregion\tmean_abs_diff\tpct_over_32\n')
        for r in rows:
            f.write('%s\t%s\t%s\t%s\t%.1f\t%.1f\n' % r)
    print('\n%-20s %-14s %-7s %-8s %6s %6s' % ('replay', 'shot', 'kind', 'region', 'mean', '>32%'))
    for r in rows:
        print('%-20s %-14s %-7s %-8s %6.1f %6.1f' % r)


def main():
    args = sys.argv[1:]
    out = '/mnt/c/shots/compare'
    mod = os.path.expanduser('~/ffx-work/demaster/scratch/compare-ui/efl/x')
    lang = 'de'
    hw, hw_scale = 'vk', 2
    rest = []
    while args:
        a = args.pop(0)
        if a == '--out':
            out = args.pop(0)
        elif a == '--mod':
            mod = args.pop(0)
        elif a == '--lang':
            lang = args.pop(0)
        elif a == '--hw':
            hw = args.pop(0)
        elif a == '--hw-scale':
            hw_scale = int(args.pop(0))
        else:
            rest.append(a)
    if not rest or rest[0] not in ('ps2', 'hw', 'pc', 'diff', 'all'):
        sys.exit(__doc__)
    cmd, names = rest[0], rest[1:]
    screens = load_screens(names)
    if cmd in ('ps2', 'all'):
        run_ps2(screens, out)
    if cmd in ('hw', 'all'):
        run_hw(screens, out, hw, hw_scale)
    if cmd in ('pc', 'all'):
        run_pc(screens, out, mod, lang)
    if cmd in ('diff', 'all'):
        run_diff(screens, out)


if __name__ == '__main__':
    main()
