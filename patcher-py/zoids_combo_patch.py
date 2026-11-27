#!/usr/bin/env python3
"""Zoids Combo Patch Kit (v1) - friends-only.

Builds "Zoids Vs. III + Battle Legends" from YOUR OWN legally-owned discs:
  * base game : Zoids Vs. III (Japan) with the T-En v1.00 English fan translation
  * donor game: Zoids: Battle Legends (USA)

No game data ships with this kit: everything is copied from your own
disc images at patch time.

What it changes (v1):
  * English pilot bios from Battle Legends (Vs. III's are still Japanese)
  * English battle voices + voice scripts from Battle Legends for every
    pilot that exists in both games

Usage:
  Put both ISOs next to this script and run it (or double-click PATCH.bat).
  Or: python zoids_combo_patch.py "Vs III.iso" "Battle Legends.iso"
  options: --out FILE   --no-voices   --no-bios   --force
"""
import argparse, hashlib, os, re, sys, textwrap, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gcdisc

KIT_VERSION = '1.2.2'

# Content fingerprints of the exact discs this kit was built against.
VS3_ID, VS3_PRINT = 'GZVJDA', '06ed3803292affa5923cf38a067c63c28103563d'
BL_ID, BL_PRINT = 'GZSE70', 'de05fac263aadcce03e800d8516e67effa78e152'
# Expected SHA-1 of the finished image (default options) - lets everyone
# confirm their build is identical, which Dolphin Netplay requires.
EXPECTED_OUT_SHA1 = '9c1161f6961798018433d8ad56f7f515d46d9f7e'

# Pilot IDs whose bios match between games (checked by hand).
# Excluded: 015/047/057 (Van & Fiona were renumbered in Vs. III) and
# 043-046 (generic soldiers Vs. III reworked).
BIO_IDS = list(range(0, 15)) + list(range(16, 43)) + list(range(59, 68))
# Pilots with a voice script in both games and the same identity.
# Also excludes 041/042 (operators: Vs. III reworked their voice sets).
VOICE_IDS = list(range(0, 15)) + list(range(16, 41)) + list(range(59, 68))

BIO_LINES, BIO_WIDTH, BIO_WRAP = 5, 50, 48

# New identity so Dolphin lists it as its own game (Dolphin names games by ID).
OUT_ID = 'GZVJZC'
OUT_DISC_NAME = 'ZOIDS COMBO VS3+BL'
BANNER = [('ZOIDS COMBO', 0x20), ('Vs.III + BL', 0x20), ('Zoids Combo: Vs. III + Battle Legends', 0x40),
          ('Fan combo mod', 0x40), ('Zoids Vs. III with Battle Legends English voices and pilot bios.', 0x80)]
FONT = {'C': ['01110', '10001', '10000', '10000', '10000', '10001', '01110'],
        'O': ['01110', '10001', '10001', '10001', '10001', '10001', '01110'],
        'M': ['10001', '11011', '10101', '10101', '10001', '10001', '10001'],
        'B': ['11110', '10001', '10001', '11110', '10001', '10001', '11110']}


def _px(d, x, y, rgb):
    t = (y // 4) * 24 + x // 4
    o = 0x20 + (t * 16 + (y % 4) * 4 + x % 4) * 2
    r, g, b = rgb
    v = 0x8000 | (r >> 3) << 10 | (g >> 3) << 5 | (b >> 3)
    d[o], d[o + 1] = v >> 8, v & 0xFF


def make_banner(src):
    """Draw a red COMBO tag on the banner and replace its text."""
    d = bytearray(src)
    for y in range(21, 32):
        for x in range(61, 96):
            _px(d, x, y, (0, 0, 0) if (y == 21 or x == 61) else (200, 16, 16))
    cx = 64
    for ch in 'COMBO':
        for r, row in enumerate(FONT[ch]):
            for c, bit in enumerate(row):
                if bit == '1':
                    _px(d, cx + c, 23 + r, (255, 255, 255))
        cx += 6
    o = 0x1820
    for text, n in BANNER:
        d[o:o + n] = text.encode('ascii').ljust(n, b'\0')
        o += n
    return bytes(d)


def fingerprint(disc):
    h = hashlib.sha1(disc.dol)
    for p in sorted(disc.files):
        h.update(p.encode('utf-8') + b'\0')
        h.update(hashlib.sha1(disc.file_data(p)).digest())
    return h.hexdigest()


def check(disc, want_id, want_print, label, force):
    if disc.game_id != want_id:
        sys.exit(f'ERROR: {label} has game ID {disc.game_id}, expected {want_id}. Wrong disc?')
    print(f'  checking {label} ...', end='', flush=True)
    fp = fingerprint(disc)
    if fp == want_print:
        print(' OK')
    else:
        print(' DIFFERENT from the reference copy')
        msg = (f'  Your {label} is not the exact version this kit expects '
               '(different dump or translation version).')
        if not force:
            sys.exit(msg + '\n  Re-run with --force to try anyway (Netplay may desync with friends).')
        print(msg + ' Continuing because of --force.')


def make_bio(text):
    words = ' '.join(l.strip() for l in text.split('\r\n')).split()
    lines = textwrap.wrap(' '.join(words), BIO_WRAP, break_on_hyphens=False, break_long_words=False)[:BIO_LINES]
    lines += [''] * (BIO_LINES - len(lines))
    return b'\r\n'.join(l.ljust(BIO_WIDTH).encode('ascii') for l in lines)


def collect_changes(vs3, bl, do_bios, do_voices):
    changes = {}
    bnr = vs3.find('OPENING.BNR')
    changes[bnr] = make_banner(vs3.file_data(bnr))
    print('  New title + banner: Zoids Combo')
    if do_bios:
        for n in BIO_IDS:
            src = bl.find(f'chara/man{n:03d}.txt')
            dst = vs3.find(f'chara/man{n:03d}.txt')
            if src and dst:
                changes[dst] = make_bio(bl.file_data(src).decode('ascii', 'replace'))
        print(f'  English pilot bios: {sum(1 for p in changes if p.startswith("chara/man"))}')
    if do_voices:
        clips = scripts = 0
        for n in VOICE_IDS:
            ssrc = bl.find(f'chara/v_sc/man_{n:02d}.sc')
            sdst = vs3.find(f'chara/v_sc/man{n:03d}.sc')
            if not (ssrc and sdst):
                continue
            changes[sdst] = bl.file_data(ssrc); scripts += 1
            pat = re.compile(rf'^voice/{n:02d}/V{n:02d}_(\d+)\.adp$', re.I)
            for p in bl.files:
                m = pat.match(p)
                if not m:
                    continue
                target = f'voice/{n:03d}/{n:03d}_{m.group(1)}.adp'
                changes[vs3.find(target) or target] = bl.file_data(p); clips += 1
        print(f'  English voice scripts: {scripts}, voice clips: {clips}')
    return changes


def main():
    ap = argparse.ArgumentParser(description='Zoids Combo Patch Kit v' + KIT_VERSION)
    ap.add_argument('discs', nargs='*', help='the two disc images in any order (auto-detected)')
    ap.add_argument('--vs3', help='Zoids Vs. III (T-En v1.00) disc image (.iso/.gcm)')
    ap.add_argument('--bl', help='Zoids: Battle Legends (USA) disc image (.iso/.gcm)')
    ap.add_argument('--out', default='Zoids Combo (Vs3 + Battle Legends).iso')
    ap.add_argument('--no-voices', action='store_true', help='keep the Japanese voices')
    ap.add_argument('--no-bios', action='store_true', help='keep the Japanese pilot bios')
    ap.add_argument('--force', action='store_true', help='patch even if a disc does not match')
    a = ap.parse_args()

    # Auto-detect: use given files, or scan the kit's folder for disc images.
    if not (a.vs3 and a.bl):
        here = os.path.dirname(os.path.abspath(__file__))
        cands = a.discs or [os.path.join(d, f) for d in (here, os.path.dirname(here))
                            for f in sorted(os.listdir(d))
                            if f.lower().endswith(('.iso', '.gcm', '.rvz'))]
        rvz = []
        for p in cands:
            if p.lower().endswith('.rvz'):
                rvz.append(os.path.basename(p))   # only matters if no ISO is found
                continue
            try:
                with open(p, 'rb') as f:
                    gid = f.read(6).decode('ascii', 'replace')
            except OSError:
                continue
            if gid == VS3_ID and not a.vs3:
                a.vs3 = p
            elif gid == BL_ID and not a.bl:
                a.bl = p
        if not (a.vs3 and a.bl) and rvz:
            sys.exit(f'ERROR: found compressed game(s) {", ".join(rvz)} but not both ISOs. In Dolphin, right-click'
                     ' the game -> Convert File... -> Format: ISO, save it in this folder, then try again.')
        if not (a.vs3 and a.bl):
            sys.exit('ERROR: need both discs. Put your Zoids Vs. III (T-En) ISO and Battle Legends ISO\n'
                     '       in this folder (or pass them on the command line) and run again.\n'
                     f'       Found Vs. III: {bool(a.vs3)}   Battle Legends: {bool(a.bl)}')
    if not os.path.isabs(a.out):
        a.out = os.path.join(os.path.dirname(os.path.abspath(a.vs3)), a.out)

    for p in (a.vs3, a.bl):
        if not os.path.isfile(p):
            sys.exit(f'ERROR: file not found: {p}')
        if p.lower().endswith(('.rvz', '.wia', '.gcz', '.ciso', '.nkit.iso')):
            sys.exit(f'ERROR: {os.path.basename(p)} is compressed. In Dolphin, right-click the game ->'
                     ' Convert File... -> Format: ISO, then point this tool at the .iso.')
    if os.path.abspath(a.out) in (os.path.abspath(a.vs3), os.path.abspath(a.bl)):
        sys.exit('ERROR: --out must be a new file, not one of your originals.')

    print(f'Zoids Combo Patch Kit v{KIT_VERSION}')
    t = time.time()
    vs3, bl = gcdisc.Disc(a.vs3), gcdisc.Disc(a.bl)
    check(vs3, VS3_ID, VS3_PRINT, 'Zoids Vs. III', a.force)
    check(bl, BL_ID, BL_PRINT, 'Battle Legends', a.force)

    print('Collecting changes from your discs:')
    changes = collect_changes(vs3, bl, not a.no_bios, not a.no_voices)

    print(f'Writing {a.out} ...')
    gcdisc.build(vs3, changes, a.out, game_id=OUT_ID, game_name=OUT_DISC_NAME,
                 progress=lambda i, n: print(f'  {100 * i // n:3d}%', end='\r', flush=True))
    print('  100%')

    h = hashlib.sha1()
    with open(a.out, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 22), b''):
            h.update(chunk)
    digest = h.hexdigest()
    print(f'Done in {time.time() - t:.0f}s.  SHA-1: {digest}')
    if not (a.no_voices or a.no_bios or a.force):
        if digest == EXPECTED_OUT_SHA1:
            print('Matches the reference build - you are Netplay-ready.')
        else:
            print('WARNING: does not match the reference build. Netplay with friends may desync.')
    else:
        print('Custom options used: compare this SHA-1 with your friends before Netplay.')


if __name__ == '__main__':
    main()
