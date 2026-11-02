import struct
import numpy as np

def batches(path):
    """Yield (positions, uvs) per VU1 batch (data between MSCAL/MSCNT codes) whose counts agree."""
    b = open(path, 'rb').read()
    s = struct.unpack_from('<I', b, 0x14)[0]
    n = struct.unpack_from('<I', b, s + 0xc)[0]
    o = s + 0x40
    out = []; skipped = 0
    for _ in range(n):
        k, ln = struct.unpack_from('<II', b, o)
        d = o + 0x40
        if k == 1:
            q, end = d + 0x40, d + ln
            P, U = [], []
            def flush():
                nonlocal P, U, skipped
                if P and len(P) == len(U):
                    out.append((np.array(P), np.array(U, float) / 4096))
                elif P or U:
                    skipped += max(len(P), len(U))
                P, U = [], []
            while q + 4 <= end:
                w = struct.unpack_from('<I', b, q)[0]
                cmd, num, imm = (w >> 24) & 0x7f, ((w >> 16) & 0xff) or 256, w & 0xffff
                q += 4
                if cmd >= 0x60:
                    vn, vl = (cmd >> 2) & 3, cmd & 3
                    size = ((32 >> vl) * (vn + 1) * num + 31) // 32 * 4
                    if (vn, vl) == (2, 0):
                        P += list(struct.iter_unpack('<3f', b[q:q + 12 * num]))
                    elif (vn, vl) == (1, 1):
                        U += list(struct.iter_unpack('<2h', b[q:q + 4 * num]))
                    q += size
                elif cmd in (0x14, 0x15, 0x17):
                    flush()
                elif cmd in (0x50, 0x51):
                    q += (imm or 0x10000) * 16
                elif cmd in (0x30, 0x31):
                    q += 16
                elif cmd == 0x20:
                    q += 4
                elif cmd == 0x4a:
                    q += num * 8
            flush()
        o += 0x40 + ln
    return out, skipped
