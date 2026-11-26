"""Minimal pure-Python GameCube disc (ISO/GCM) reader and rebuilder.

No third-party dependencies. Reads the FST, lets you replace/add files,
and writes a fresh, deterministic, full-size disc image.
"""
import struct

DISC_SIZE = 1459978240          # standard full-size GameCube disc
ALIGN = 0x8000                  # file alignment (matches retail layout)


def _align(x, a):
    return (x + a - 1) // a * a


class Disc:
    def __init__(self, path):
        self.path = path
        self.f = open(path, 'rb')
        hdr = self.read(0, 0x440)
        self.game_id = hdr[0:6].decode('ascii', 'replace')
        self.dol_off, self.fst_off, self.fst_size, self.fst_max = struct.unpack('>4I', hdr[0x420:0x430])
        self.boot = hdr
        self.bi2 = self.read(0x440, 0x2000)
        # apploader: header 0x20 bytes, size at +0x14, trailer at +0x18
        ah = self.read(0x2440, 0x20)
        asz, atr = struct.unpack('>II', ah[0x14:0x1C])
        self.apploader = self.read(0x2440, _align(0x20 + asz + atr, 0x20))
        self.dol = self.read(self.dol_off, self._dol_size())
        self.files = self._parse_fst(self.read(self.fst_off, self.fst_size))

    def read(self, off, n):
        self.f.seek(off)
        return self.f.read(n)

    def _dol_size(self):
        h = self.read(self.dol_off, 0x100)
        offs = struct.unpack('>18I', h[0:0x48])
        sizes = struct.unpack('>18I', h[0x90:0xD8])
        return max(o + s for o, s in zip(offs, sizes) if o)

    def _parse_fst(self, fst):
        n = struct.unpack('>I', fst[8:12])[0]
        strs = n * 12

        def name(i):
            o = struct.unpack('>I', fst[i * 12:i * 12 + 4])[0] & 0xFFFFFF
            e = fst.index(b'\0', strs + o)
            return fst[strs + o:e].decode('shift_jis')

        files = {}   # path -> (offset, size)  ; ordered as on disc FST
        self.dirs = []
        stack = [(n, '')]
        i = 1
        while i < n:
            while stack and i >= stack[-1][0]:
                stack.pop()
            flags = fst[i * 12]
            a, b = struct.unpack('>II', fst[i * 12 + 4:i * 12 + 12])
            path = stack[-1][1] + name(i)
            if flags:
                self.dirs.append(path)
                stack.append((b, path + '/'))
            else:
                files[path] = (a, b)
            i += 1
        return files

    def file_data(self, path):
        o, s = self.files[path]
        return self.read(o, s)

    def find(self, path):
        """Case-insensitive lookup (the console's FST is case-insensitive)."""
        if path in self.files:
            return path
        low = path.lower()
        for p in self.files:
            if p.lower() == low:
                return p
        return None


def build(base, changes, out_path, progress=None, game_id=None, game_name=None):
    """Write a new disc from `base` with `changes` applied.

    changes: dict path -> bytes. Path matching an existing file (any case)
    replaces it; a new path adds a file (parent dirs are created).
    """
    # merge file list, keep original order, then new files sorted
    entries = {}
    for p in base.files:
        entries[p] = ('base', p)
    for p, data in changes.items():
        hit = base.find(p)
        entries[hit or p] = ('new', data)

    # build tree
    tree = {}
    for d in base.dirs:
        node = tree
        for part in d.split('/'):
            node = node.setdefault(part, {})
    for p in entries:
        parts = p.split('/')
        node = tree
        for part in parts[:-1]:
            node = node.setdefault(part, {})
        node[parts[-1]] = p   # leaf -> full path

    # GameCube FST requires entries sorted case-insensitively within a dir
    def key(n):
        return n.lower()

    flat = []   # (is_dir, name, full_path_or_None, parent_index, end_index placeholder)

    def walk(node, parent):
        for nm in sorted(node, key=key):
            v = node[nm]
            if isinstance(v, dict):
                idx = len(flat)
                flat.append([True, nm, None, parent, 0])
                walk(v, idx + 1)   # FST index = flat index + 1 (root is 0)
                flat[idx][4] = len(flat) + 1
            else:
                flat.append([False, nm, v, parent, 0])
    walk(tree, 0)

    # string table
    strtab = bytearray()
    name_off = []
    for e in flat:
        name_off.append(len(strtab))
        strtab += e[1].encode('shift_jis') + b'\0'
    n = len(flat) + 1
    fst_size = n * 12 + len(strtab)

    dol_off = _align(0x2440 + len(base.apploader), ALIGN)
    fst_off = _align(dol_off + len(base.dol), ALIGN)
    pos = _align(fst_off + fst_size, ALIGN)

    # assign file offsets in FST order
    offsets = {}
    for e in flat:
        if not e[0]:
            src = entries[e[2]]
            size = base.files[src[1]][1] if src[0] == 'base' else len(src[1])
            offsets[e[2]] = (pos, size)
            pos = _align(pos + size, ALIGN)
    if pos > DISC_SIZE:
        raise RuntimeError('Patched game does not fit on a GameCube disc')

    fst = bytearray(struct.pack('>BBHII', 1, 0, 0, 0, n))
    for i, e in enumerate(flat):
        if e[0]:
            fst += struct.pack('>I', 0x01000000 | name_off[i]) + struct.pack('>II', e[3], e[4])
        else:
            o, s = offsets[e[2]]
            fst += struct.pack('>III', name_off[i], o, s)
    fst += strtab

    boot = bytearray(base.boot)
    struct.pack_into('>4I', boot, 0x420, dol_off, fst_off, fst_size, max(fst_size, base.fst_max))
    if game_id:
        boot[0:6] = game_id.encode('ascii')
    if game_name:
        boot[0x20:0x400] = game_name.encode('ascii').ljust(0x3E0, b'\0')

    with open(out_path, 'wb') as out:
        out.write(boot)
        out.write(base.bi2)
        out.write(base.apploader)
        out.seek(dol_off); out.write(base.dol)
        out.seek(fst_off); out.write(fst)
        total = len(offsets)
        for k, e in enumerate(flat):
            if e[0]:
                continue
            src = entries[e[2]]
            data = base.file_data(src[1]) if src[0] == 'base' else src[1]
            out.seek(offsets[e[2]][0]); out.write(data)
            if progress and k % 500 == 0:
                progress(k, len(flat))
        out.truncate(DISC_SIZE)
