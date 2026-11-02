"""Minimal PS2 GS local-memory emulation (swizzle layouts), enough to
reinterpret texture data uploaded in one pixel format and read in another.

Addresses follow GSdx/PCSX2 GSLocalMemory: bp = block pointer (64-word
blocks), bw = buffer width in 64-pixel units. A page is 32 blocks (8 KiB).
"""

MEM_SIZE = 4 * 1024 * 1024

# Block order inside a page.
BLOCK32 = [
    [0, 1, 4, 5, 16, 17, 20, 21],
    [2, 3, 6, 7, 18, 19, 22, 23],
    [8, 9, 12, 13, 24, 25, 28, 29],
    [10, 11, 14, 15, 26, 27, 30, 31],
]
BLOCK8 = BLOCK32  # 8x4 blocks of 16x16 texels per 128x64 page
BLOCK4 = [
    [0, 2, 8, 10],
    [1, 3, 9, 11],
    [4, 6, 12, 14],
    [5, 7, 13, 15],
    [16, 18, 24, 26],
    [17, 19, 25, 27],
    [20, 22, 28, 30],
    [21, 23, 29, 31],
]

# Word index inside an 8x8 PSMCT32 block.
COLUMN32 = [
    [0, 1, 4, 5, 8, 9, 12, 13],
    [2, 3, 6, 7, 10, 11, 14, 15],
    [16, 17, 20, 21, 24, 25, 28, 29],
    [18, 19, 22, 23, 26, 27, 30, 31],
    [32, 33, 36, 37, 40, 41, 44, 45],
    [34, 35, 38, 39, 42, 43, 46, 47],
    [48, 49, 52, 53, 56, 57, 60, 61],
    [50, 51, 54, 55, 58, 59, 62, 63],
]

# Byte index inside a 16x16 PSMT8 block.
_C8 = [
    [0, 4, 16, 20, 32, 36, 48, 52, 2, 6, 18, 22, 34, 38, 50, 54],
    [8, 12, 24, 28, 40, 44, 56, 60, 10, 14, 26, 30, 42, 46, 58, 62],
    [33, 37, 49, 53, 1, 5, 17, 21, 35, 39, 51, 55, 3, 7, 19, 23],
    [41, 45, 57, 61, 9, 13, 25, 29, 43, 47, 59, 63, 11, 15, 27, 31],
    [96, 100, 112, 116, 64, 68, 80, 84, 98, 102, 114, 118, 66, 70, 82, 86],
    [104, 108, 120, 124, 72, 76, 88, 92, 106, 110, 122, 126, 74, 78, 90, 94],
    [65, 69, 81, 85, 97, 101, 113, 117, 67, 71, 83, 87, 99, 103, 115, 119],
    [73, 77, 89, 93, 105, 109, 121, 125, 75, 79, 91, 95, 107, 111, 123, 127],
]
COLUMN8 = _C8 + [[v + 128 for v in row] for row in _C8]

# Nibble index inside a 32x16 PSMT4 block.
_C4 = [
    [0, 8, 32, 40, 64, 72, 96, 104, 2, 10, 34, 42, 66, 74, 98, 106,
     4, 12, 36, 44, 68, 76, 100, 108, 6, 14, 38, 46, 70, 78, 102, 110],
    [16, 24, 48, 56, 80, 88, 112, 120, 18, 26, 50, 58, 82, 90, 114, 122,
     20, 28, 52, 60, 84, 92, 116, 124, 22, 30, 54, 62, 86, 94, 118, 126],
    [65, 73, 97, 105, 1, 9, 33, 41, 67, 75, 99, 107, 3, 11, 35, 43,
     69, 77, 101, 109, 5, 13, 37, 45, 71, 79, 103, 111, 7, 15, 39, 47],
    [81, 89, 113, 121, 17, 25, 49, 57, 83, 91, 115, 123, 19, 27, 51, 59,
     85, 93, 117, 125, 21, 29, 53, 61, 87, 95, 119, 127, 23, 31, 55, 63],
    [192, 200, 224, 232, 128, 136, 160, 168, 194, 202, 226, 234, 130, 138, 162, 170,
     196, 204, 228, 236, 132, 140, 164, 172, 198, 206, 230, 238, 134, 142, 166, 174],
    [208, 216, 240, 248, 144, 152, 176, 184, 210, 218, 242, 250, 146, 154, 178, 186,
     212, 220, 244, 252, 148, 156, 180, 188, 214, 222, 246, 254, 150, 158, 182, 190],
    [129, 137, 161, 169, 193, 201, 225, 233, 131, 139, 163, 171, 195, 203, 227, 235,
     133, 141, 165, 173, 197, 205, 229, 237, 135, 143, 167, 175, 199, 207, 231, 239],
    [145, 153, 177, 185, 209, 217, 241, 249, 147, 155, 179, 187, 211, 219, 243, 251,
     149, 157, 181, 189, 213, 221, 245, 253, 151, 159, 183, 191, 215, 223, 247, 255],
]
COLUMN4 = _C4 + [[v + 256 for v in row] for row in _C4]


def addr32(x, y, bp, bw):
    """Word address of a PSMCT32 texel."""
    page = (y >> 5) * bw + (x >> 6)
    block = bp + page * 32 + BLOCK32[(y >> 3) & 3][(x >> 3) & 7]
    return (block << 6) + COLUMN32[y & 7][x & 7]


def addr8(x, y, bp, bw):
    """Byte address of a PSMT8 texel (bw in 64-texel units, even)."""
    page = (y >> 6) * (bw >> 1) + (x >> 7)
    block = bp + page * 32 + BLOCK8[(y >> 4) & 3][(x >> 4) & 7]
    return (block << 8) + COLUMN8[y & 15][x & 15]


def addr4(x, y, bp, bw):
    """Nibble address of a PSMT4 texel (bw in 64-texel units, even)."""
    page = (y >> 7) * (bw >> 1) + (x >> 7)
    block = bp + page * 32 + BLOCK4[(y >> 4) & 7][(x >> 5) & 3]
    return (block << 9) + COLUMN4[y & 15][x & 31]


class GSMem:
    def __init__(self):
        self.m = bytearray(MEM_SIZE)

    def write32(self, bp, bw, w, h, data):
        """Host->local transfer in PSMCT32 (data is linear RGBA words)."""
        m = self.m
        i = 0
        for y in range(h):
            for x in range(w):
                a = (addr32(x, y, bp, bw) * 4) % MEM_SIZE
                m[a:a + 4] = data[i:i + 4]
                i += 4

    def write8(self, bp, bw, w, h, data):
        m = self.m
        i = 0
        for y in range(h):
            for x in range(w):
                m[addr8(x, y, bp, bw) % MEM_SIZE] = data[i]
                i += 1

    def read8(self, bp, bw, w, h):
        m = self.m
        return bytes(m[addr8(x, y, bp, bw) % MEM_SIZE] for y in range(h) for x in range(w))

    def read4(self, bp, bw, w, h):
        m = self.m
        out = bytearray(w * h)
        i = 0
        for y in range(h):
            for x in range(w):
                a = addr4(x, y, bp, bw) % (MEM_SIZE * 2)
                v = m[a >> 1]
                out[i] = (v >> 4) if a & 1 else (v & 15)
                i += 1
        return bytes(out)

    def read4h(self, bp, bw, w, h, high):
        """PSMT4HL (high=False) / PSMT4HH (high=True): bits 24..27 / 28..31 of a CT32 word."""
        m = self.m
        sh = 4 if high else 0
        return bytes((m[(addr32(x, y, bp, bw) * 4 + 3) % MEM_SIZE] >> sh) & 15
                     for y in range(h) for x in range(w))


def unpack4_linear(data, w, h):
    """Linear 4bpp, low nibble = left texel (GS host transfer order)."""
    out = bytearray(w * h)
    for i in range(w * h):
        v = data[i >> 1]
        out[i] = (v >> 4) if i & 1 else (v & 15)
    return bytes(out)


def ps2_alpha(a):
    """GS alpha: 0x80 = 1.0."""
    return 255 if a >= 0x80 else (a * 255 + 64) // 128


def csm1_index(i):
    """CSM1 CLUT entry order for a 256-colour palette uploaded as a 16x16 CT32
    rectangle: swap index bits 3 and 4."""
    return (i & 0xE7) | ((i & 0x08) << 1) | ((i & 0x10) >> 1)


def palette_rgba(raw, count=256, csm1=True):
    """raw: linear RGBA8888 entries as stored/uploaded. Returns list of (r,g,b,a)
    indexed by texel value."""
    pal = []
    for i in range(count):
        j = csm1_index(i) if (csm1 and count == 256) else i
        r, g, b, a = raw[4 * j:4 * j + 4]
        pal.append((r, g, b, ps2_alpha(a)))
    return pal


def palette16_from_256(raw256, csa):
    """16-colour slice CSA of a 256-entry CSM1 CLUT (CT32), as used by a PSMT4
    TEX0 with CSA. Buffer entry e = csa*16 + k lives at rect entry csm1(e)."""
    pal = []
    for k in range(16):
        j = csm1_index(csa * 16 + k)
        r, g, b, a = raw256[4 * j:4 * j + 4]
        pal.append((r, g, b, ps2_alpha(a)))
    return pal
