"""Sets up a portable PCSX2 for tools/compare (docs/compare.md).

- PCSX2.ini: memory card slot 1 = the folder card `ffx-compare.ps2`, pad 1
  bound to the keys pcsx2ctl posts, the screenshot hotkey (F12) writing the
  GS output at the internal resolution as PNG, the single-frame GS dump
  hotkey (F11, zstd), no pause on focus loss; the software renderer at
  native resolution.
- `memcards/ffx-compare.ps2/`: a PCSX2 folder memory card holding the PC
  saves as PS2 saves. The PC save file is the PS2 one with zero padding
  (0x64f8 bytes used of 0x6900, same CRC: docs/compare.md), so the save
  slot n is `BESCES-50492FF0906nn/BESCES-50492FF0906nn` = the first 0x64f8
  bytes of `ffx_0nn`.

usage: python -I setup.py <pcsx2 dir> <PC save dir> [slot...]   (default slots 1 5)
       python -I setup.py renderer <pcsx2 dir> sw|vk|dx12|dx11 [scale]
         (compare.py switches the renderer this way for the GS dump replays)
"""
import os
import re
import struct
import sys

PAD = {
    'Type': 'DualShock2',
    'Up': 'Keyboard/Up', 'Down': 'Keyboard/Down', 'Left': 'Keyboard/Left', 'Right': 'Keyboard/Right',
    'Cross': 'Keyboard/K', 'Circle': 'Keyboard/L', 'Square': 'Keyboard/J', 'Triangle': 'Keyboard/I',
    'Start': 'Keyboard/Return', 'Select': 'Keyboard/Backspace',
    'L1': 'Keyboard/Q', 'R1': 'Keyboard/E', 'L2': 'Keyboard/1', 'R2': 'Keyboard/3',
}
SETTINGS = {
    'MemoryCards': {'Slot1_Enable': 'true', 'Slot1_Filename': 'ffx-compare.ps2',
                    'Slot2_Enable': 'false'},
    'Pad1': PAD,
    'Hotkeys': {'Screenshot': 'Keyboard/F12', 'GSDumpSingleFrame': 'Keyboard/F11'},
    # 2 = internal resolution, no aspect correction: the PS2 frame as the GS
    # outputs it; format 0 = PNG. GS dumps zstd (2).
    'EmuCore/GS': {'ScreenshotSize': '2', 'ScreenshotFormat': '0', 'GSDumpCompression': '2',
                   'Renderer': '13', 'upscale_multiplier': '1'},
    'UI': {'PauseOnFocusLoss': 'false', 'ConfirmShutdown': 'false'},
    'EmuCore': {'McdFolderAutoManage': 'true'},
}

# PCSX2's GSRendererType
RENDERERS = {'sw': '13', 'vk': '14', 'dx12': '15', 'dx11': '3'}

SERIAL = 'BESCES-50492'
SIZE = 0x64f8


def set_ini(path, settings=None):
    text = open(path, encoding='utf-8').read().replace('\r\n', '\n')
    for section, kv in (settings or SETTINGS).items():
        m = re.search(r'^\[' + re.escape(section) + r'\]\n', text, re.M)
        if not m:
            text = text.rstrip('\n') + '\n\n[' + section + ']\n'
            m = re.search(r'^\[' + re.escape(section) + r'\]\n', text, re.M)
        start = m.end()
        nxt = re.search(r'^\[', text[start:], re.M)
        end = start + nxt.start() if nxt else len(text)
        body = text[start:end]
        for k, v in kv.items():
            line = f'{k} = {v}'
            if re.search(r'^' + re.escape(k) + r' = .*$', body, re.M):
                body = re.sub(r'^' + re.escape(k) + r' = .*$', line, body, flags=re.M)
            else:
                body = body.rstrip('\n') + ('\n' if body.strip() else '') + line + '\n'
        if nxt and not body.endswith('\n\n'):
            body = body.rstrip('\n') + '\n\n'
        text = text[:start] + body + text[end:]
    open(path, 'w', encoding='utf-8', newline='\r\n').write(text)


def superblock():
    """A formatted 8 MB card's superblock. PCSX2's folder card adds the
    save folders only when `_pcsx2_superblock` says the card is formatted
    (`FolderMemoryCard::LoadMemoryCardData`); it builds the FAT itself."""
    sb = bytearray(b'\xff' * 0x2000)
    struct.pack_into('<28s12sHHHHIIIIII', sb, 0,
                     b'Sony PS2 Memory Card Format ', b'1.2.0.0\0\0\0\0\0',
                     512, 2, 16, 0xff00, 8192, 41, 8135, 0, 1023, 1022)
    sb[0x48:0x50] = bytes(8)
    ifc = [8] + [0] * 31
    struct.pack_into('<32I', sb, 0x50, *ifc)
    struct.pack_into('<32I', sb, 0xd0, *([0xffffffff] * 32))
    sb[0x150] = 2      # card type: PS2
    sb[0x151] = 0x52   # card flags
    return bytes(sb)


def make_card(card, saves, slots):
    os.makedirs(card, exist_ok=True)
    sbf = os.path.join(card, '_pcsx2_superblock')
    if not os.path.exists(sbf):
        open(sbf, 'wb').write(superblock())
    for n in slots:
        src = os.path.join(saves, 'ffx_%03d' % n)
        data = open(src, 'rb').read()
        if len(data) < SIZE or any(data[SIZE:]):
            sys.exit(f'{src}: not a PC save with the PS2 layout')
        name = '%sFF0906%02d' % (SERIAL, n)
        d = os.path.join(card, name)
        os.makedirs(d, exist_ok=True)
        open(os.path.join(d, name), 'wb').write(data[:SIZE])
        print('card:', name, '<-', src)


def main():
    if len(sys.argv) >= 4 and sys.argv[1] == 'renderer':
        scale = sys.argv[4] if len(sys.argv) > 4 else '1'
        set_ini(os.path.join(sys.argv[2], 'inis', 'PCSX2.ini'),
                {'EmuCore/GS': {'Renderer': RENDERERS[sys.argv[3]], 'upscale_multiplier': scale}})
        return
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    pcsx2, saves = sys.argv[1], sys.argv[2]
    slots = [int(s) for s in sys.argv[3:]] or [1, 5]
    set_ini(os.path.join(pcsx2, 'inis', 'PCSX2.ini'))
    make_card(os.path.join(pcsx2, 'memcards', 'ffx-compare.ps2'), saves, slots)


if __name__ == '__main__':
    main()
