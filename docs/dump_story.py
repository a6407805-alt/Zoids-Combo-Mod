import struct, os, sys
D = sys.argv[1]
t = open(os.path.join(D, next(x for x in os.listdir(D) if x.upper() == 'TEXT.BIN')), 'rb').read()
n = struct.unpack('>I', t[:4])[0] // 4
offs = list(struct.unpack(f'>{n}I', t[:4 * n])) + [len(t)]
def msg(i):
    if i >= n: return f'<msg {i}?>'
    return t[offs[i]:offs[i + 1]].decode('latin-1').replace('\r\n', ' ').strip()
scenes = {}
for f in sorted(os.listdir(D)):
    if not (f.upper().startswith('DEMO_') and f.upper().endswith('.BIN')): continue
    num = int(f[5:8]); d = open(os.path.join(D, f), 'rb').read()
    recs = []; i = 0; exp = 0
    while i + 22 <= len(d):
        h = struct.unpack('>11H', d[i:i + 22])
        if h[0] != exp: break
        recs.append(h); exp += 1; i += 22
    scenes[num] = (recs, d[i:])
import json
out = {}
for num, (recs, tail) in sorted(scenes.items()):
    lines = [msg(r[6]) for r in recs if r[6] < 0xfff0]
    out[num] = {'n': len(recs), 'bg': sorted({r[5] for r in recs}), 'text': lines, 'tail': tail.hex()}
json.dump(out, open(sys.argv[2], 'w'), indent=0)
print(len(out), 'scenes; total msgs', n)
