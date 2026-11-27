//! Zoids Combo Patch Kit - standalone build (no Python needed).
//! Byte-for-byte the same output as zoids_combo_patch.py.
#[cfg(feature = "experimental")]
mod story;
#[cfg(feature = "experimental")]
mod music;
#[cfg(feature = "experimental")]
mod gauntlet;
/// Gauntlet run seed (set with --seed).
static SEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const KIT_VERSION: &str = "1.2.2";
const VS3_ID: &str = "GZVJDA";
const VS3_PRINT: &str = "06ed3803292affa5923cf38a067c63c28103563d";
const BL_ID: &str = "GZSE70";
const BL_PRINT: &str = "de05fac263aadcce03e800d8516e67effa78e152";
const EXPECTED_OUT_SHA1: &str = "9c1161f6961798018433d8ad56f7f515d46d9f7e";
const OUT_ID: &str = "GZVJZC";
const OUT_DISC_NAME: &str = "ZOIDS COMBO VS3+BL";
const BANNER: [(&str, usize); 5] = [("ZOIDS COMBO", 0x20), ("Vs.III + BL", 0x20), ("Zoids Combo: Vs. III + Battle Legends", 0x40),
    ("Fan combo mod", 0x40), ("Zoids Vs. III with Battle Legends English voices and pilot bios.", 0x80)];

fn glyph(c: char) -> [&'static str; 7] {
    match c {
        'C' => ["01110", "10001", "10000", "10000", "10000", "10001", "01110"],
        'O' => ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
        'M' => ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
        _ => ["11110", "10001", "10001", "11110", "10001", "10001", "11110"], // B
    }
}

fn px(d: &mut [u8], x: usize, y: usize, rgb: (u16, u16, u16)) {
    let t = (y / 4) * 24 + x / 4;
    let o = 0x20 + (t * 16 + (y % 4) * 4 + x % 4) * 2;
    let v: u16 = 0x8000 | (rgb.0 >> 3) << 10 | (rgb.1 >> 3) << 5 | (rgb.2 >> 3);
    d[o] = (v >> 8) as u8; d[o + 1] = (v & 0xFF) as u8;
}

fn make_banner(src: &[u8]) -> Vec<u8> {
    let mut d = src.to_vec();
    for y in 21..32 { for x in 61..96 {
        px(&mut d, x, y, if y == 21 || x == 61 { (0, 0, 0) } else { (200, 16, 16) });
    } }
    let mut cx = 64;
    for ch in "COMBO".chars() {
        for (r, row) in glyph(ch).iter().enumerate() {
            for (c, bit) in row.chars().enumerate() { if bit == '1' { px(&mut d, cx + c, 23 + r, (255, 255, 255)); } }
        }
        cx += 6;
    }
    let mut o = 0x1820;
    for (text, n) in BANNER {
        let mut f = vec![0u8; n]; f[..text.len()].copy_from_slice(text.as_bytes());
        d[o..o + n].copy_from_slice(&f); o += n;
    }
    d
}
const DISC_SIZE: u64 = 1459978240;
const ALIGN: u64 = 0x8000;
const BIO_LINES: usize = 5;
const BIO_WIDTH: usize = 50;
const BIO_WRAP: usize = 48;

fn bio_ids() -> Vec<u32> { (0..15).chain(16..43).chain(59..68).collect() }
fn voice_ids() -> Vec<u32> { (0..15).chain(16..41).chain(59..68).collect() }

fn align(x: u64, a: u64) -> u64 { (x + a - 1) / a * a }
fn be32(b: &[u8], o: usize) -> u32 { u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }
fn hex(d: &[u8]) -> String { d.iter().map(|b| format!("{:02x}", b)).collect() }

struct Disc {
    f: File,
    game_id: String,
    fst_max: u32,
    boot: Vec<u8>,
    bi2: Vec<u8>,
    apploader: Vec<u8>,
    dol: Vec<u8>,
    order: Vec<String>,                       // file paths, FST order
    files: HashMap<String, (u64, u64)>,       // path -> (offset, size)
    raw: HashMap<String, Vec<u8>>,            // path (file or dir) -> raw name bytes of last component
    dirs: Vec<String>,
}

impl Disc {
    fn open(path: &Path) -> io::Result<Disc> {
        let mut d = Disc {
            f: File::open(path)?, game_id: String::new(), fst_max: 0, boot: vec![], bi2: vec![],
            apploader: vec![], dol: vec![], order: vec![], files: HashMap::new(), raw: HashMap::new(), dirs: vec![],
        };
        let hdr = d.read(0, 0x440)?;
        d.game_id = String::from_utf8_lossy(&hdr[0..6]).into_owned();
        let (dol_off, fst_off, fst_size) = (be32(&hdr, 0x420) as u64, be32(&hdr, 0x424) as u64, be32(&hdr, 0x428) as u64);
        d.fst_max = be32(&hdr, 0x42C);
        d.boot = hdr;
        d.bi2 = d.read(0x440, 0x2000)?;
        let ah = d.read(0x2440, 0x20)?;
        let (asz, atr) = (be32(&ah, 0x14) as u64, be32(&ah, 0x18) as u64);
        d.apploader = d.read(0x2440, align(0x20 + asz + atr, 0x20))?;
        let h = d.read(dol_off, 0x100)?;
        let mut dsz = 0u64;
        for i in 0..18 {
            let (o, s) = (be32(&h, i * 4) as u64, be32(&h, 0x90 + i * 4) as u64);
            if o != 0 { dsz = dsz.max(o + s); }
        }
        d.dol = d.read(dol_off, dsz)?;
        let fst = d.read(fst_off, fst_size)?;
        d.parse_fst(&fst);
        Ok(d)
    }

    fn read(&mut self, off: u64, n: u64) -> io::Result<Vec<u8>> {
        self.f.seek(SeekFrom::Start(off))?;
        let mut v = vec![0u8; n as usize];
        self.f.read_exact(&mut v)?;
        Ok(v)
    }

    fn parse_fst(&mut self, fst: &[u8]) {
        let n = be32(fst, 8) as usize;
        let strs = n * 12;
        let name = |i: usize| -> Vec<u8> {
            let o = (be32(fst, i * 12) & 0xFFFFFF) as usize + strs;
            let e = fst[o..].iter().position(|&b| b == 0).unwrap() + o;
            fst[o..e].to_vec()
        };
        let mut stack: Vec<(usize, String)> = vec![(n, String::new())];
        let mut i = 1;
        while i < n {
            while !stack.is_empty() && i >= stack.last().unwrap().0 { stack.pop(); }
            let flags = fst[i * 12];
            let (a, b) = (be32(fst, i * 12 + 4) as u64, be32(fst, i * 12 + 8) as u64);
            let rawn = name(i);
            let (dec, _, _) = encoding_rs::SHIFT_JIS.decode(&rawn);
            // match Python's strict shift_jis codec (encoding_rs is Windows-31J)
            let dec: String = dec.chars().map(|c| match c {
                '\u{FF5E}' => '\u{301C}', '\u{2225}' => '\u{2016}', '\u{FF0D}' => '\u{2212}',
                '\u{FFE0}' => '\u{00A2}', '\u{FFE1}' => '\u{00A3}', '\u{FFE2}' => '\u{00AC}', c => c,
            }).collect();
            let path = format!("{}{}", stack.last().unwrap().1, dec);
            self.raw.insert(path.clone(), rawn);
            if flags != 0 {
                self.dirs.push(path.clone());
                stack.push((b as usize, path + "/"));
            } else {
                self.order.push(path.clone());
                self.files.insert(path, (a, b));
            }
            i += 1;
        }
    }

    fn data(&mut self, p: &str) -> io::Result<Vec<u8>> {
        let (o, s) = self.files[p];
        self.read(o, s)
    }

    fn find(&self, p: &str) -> Option<String> {
        if self.files.contains_key(p) { return Some(p.to_string()); }
        let low = p.to_lowercase();
        self.order.iter().find(|q| q.to_lowercase() == low).cloned()
    }
}

enum Src { Base(String), New(Vec<u8>) }

struct Node { name: String, raw: Vec<u8>, kids: Vec<Node>, leaf: Option<String> }

fn child<'a>(node: &'a mut Node, name: &str, raw: Vec<u8>) -> &'a mut Node {
    let pos = node.kids.iter().position(|k| k.name == name && k.leaf.is_none());
    let idx = match pos {
        Some(i) => i,
        None => { node.kids.push(Node { name: name.to_string(), raw, kids: vec![], leaf: None }); node.kids.len() - 1 }
    };
    &mut node.kids[idx]
}

fn raw_for(base: &Disc, path: &str, comp: &str) -> Vec<u8> {
    base.raw.get(path).cloned().unwrap_or_else(|| encoding_rs::SHIFT_JIS.encode(comp).0.into_owned())
}

fn build(base: &mut Disc, changes: &[(String, Vec<u8>)], out_path: &Path) -> io::Result<()> {
    let mut keys: Vec<String> = base.order.clone();
    let mut entries: HashMap<String, Src> = base.order.iter().map(|p| (p.clone(), Src::Base(p.clone()))).collect();
    for (p, data) in changes {
        let k = base.find(p).unwrap_or_else(|| p.clone());
        if !entries.contains_key(&k) { keys.push(k.clone()); }
        entries.insert(k, Src::New(data.clone()));
    }
    let mut root = Node { name: String::new(), raw: vec![], kids: vec![], leaf: None };
    for d in base.dirs.clone() {
        let mut node = &mut root;
        let mut acc = String::new();
        for part in d.split('/') {
            acc = if acc.is_empty() { part.to_string() } else { format!("{}/{}", acc, part) };
            node = child(node, part, raw_for(base, &acc, part));
        }
    }
    for p in &keys {
        let parts: Vec<&str> = p.split('/').collect();
        let mut node = &mut root;
        let mut acc = String::new();
        for part in &parts[..parts.len() - 1] {
            acc = if acc.is_empty() { part.to_string() } else { format!("{}/{}", acc, part) };
            node = child(node, part, raw_for(base, &acc, part));
        }
        let last = parts[parts.len() - 1];
        node.kids.push(Node { name: last.to_string(), raw: raw_for(base, p, last), kids: vec![], leaf: Some(p.clone()) });
    }
    // flatten: (is_dir, raw_name, leaf path, parent, end)
    let mut flat: Vec<(bool, Vec<u8>, Option<String>, u32, u32)> = vec![];
    fn walk(node: &mut Node, parent: u32, flat: &mut Vec<(bool, Vec<u8>, Option<String>, u32, u32)>) {
        node.kids.sort_by_key(|k| k.name.to_lowercase()); // stable, like Python's sorted
        for k in node.kids.iter_mut() {
            if k.leaf.is_none() {
                let idx = flat.len();
                flat.push((true, k.raw.clone(), None, parent, 0));
                walk(k, idx as u32 + 1, flat);
                flat[idx].4 = flat.len() as u32 + 1;
            } else {
                flat.push((false, k.raw.clone(), k.leaf.clone(), parent, 0));
            }
        }
    }
    walk(&mut root, 0, &mut flat);

    let mut strtab: Vec<u8> = vec![];
    let mut name_off = vec![];
    for e in &flat { name_off.push(strtab.len() as u32); strtab.extend_from_slice(&e.1); strtab.push(0); }
    let n = flat.len() as u32 + 1;
    let fst_size = n as u64 * 12 + strtab.len() as u64;
    let dol_off = align(0x2440 + base.apploader.len() as u64, ALIGN);
    let fst_off = align(dol_off + base.dol.len() as u64, ALIGN);
    let mut pos = align(fst_off + fst_size, ALIGN);
    let mut offsets: HashMap<String, (u64, u64)> = HashMap::new();
    for e in &flat {
        if let Some(p) = &e.2 {
            let size = match &entries[p] { Src::Base(b) => base.files[b].1, Src::New(d) => d.len() as u64 };
            offsets.insert(p.clone(), (pos, size));
            pos = align(pos + size, ALIGN);
        }
    }
    if pos > DISC_SIZE { return Err(io::Error::other("Patched game does not fit on a GameCube disc")); }

    let mut fst: Vec<u8> = vec![1, 0, 0, 0, 0, 0, 0, 0];
    fst.extend_from_slice(&n.to_be_bytes());
    for (i, e) in flat.iter().enumerate() {
        if e.0 {
            fst.extend_from_slice(&(0x01000000 | name_off[i]).to_be_bytes());
            fst.extend_from_slice(&e.3.to_be_bytes());
            fst.extend_from_slice(&e.4.to_be_bytes());
        } else {
            let (o, s) = offsets[e.2.as_ref().unwrap()];
            fst.extend_from_slice(&name_off[i].to_be_bytes());
            fst.extend_from_slice(&(o as u32).to_be_bytes());
            fst.extend_from_slice(&(s as u32).to_be_bytes());
        }
    }
    fst.extend_from_slice(&strtab);
    let mut boot = base.boot.clone();
    boot[0..6].copy_from_slice(OUT_ID.as_bytes());
    let mut nm = vec![0u8; 0x3E0]; nm[..OUT_DISC_NAME.len()].copy_from_slice(OUT_DISC_NAME.as_bytes());
    boot[0x20..0x400].copy_from_slice(&nm);
    for (k, v) in [dol_off as u32, fst_off as u32, fst_size as u32, (fst_size as u32).max(base.fst_max)].iter().enumerate() {
        boot[0x420 + k * 4..0x424 + k * 4].copy_from_slice(&v.to_be_bytes());
    }
    let mut out = File::create(out_path)?;
    out.write_all(&boot)?;
    out.write_all(&base.bi2)?;
    out.write_all(&base.apploader)?;
    out.seek(SeekFrom::Start(dol_off))?; out.write_all(&base.dol)?;
    out.seek(SeekFrom::Start(fst_off))?; out.write_all(&fst)?;
    let total = flat.len();
    for (k, e) in flat.iter().enumerate() {
        if let Some(p) = &e.2 {
            let data = match &entries[p] { Src::Base(b) => base.data(&b.clone())?, Src::New(d) => d.clone() };
            out.seek(SeekFrom::Start(offsets[p].0))?;
            out.write_all(&data)?;
            if k % 500 == 0 { print!("  {:3}%\r", 100 * k / total); io::stdout().flush()?; }
        }
    }
    out.set_len(DISC_SIZE)?;
    println!("  100%");
    Ok(())
}

fn fingerprint(d: &mut Disc) -> io::Result<String> {
    let mut h = Sha1::new();
    h.update(&d.dol);
    let mut paths = d.order.clone();
    paths.sort();
    for p in paths {
        h.update(p.as_bytes()); h.update([0u8]);
        let data = d.data(&p)?;
        h.update(Sha1::digest(&data));
    }
    Ok(hex(&h.finalize()))
}

fn make_bio(text: &str) -> Vec<u8> {
    let joined: Vec<&str> = text.split("\r\n").map(|l| l.trim()).collect();
    let joined = joined.join(" ");
    let mut lines: Vec<String> = vec![];
    let mut cur = String::new();
    for w in joined.split_whitespace() {
        if cur.is_empty() { cur = w.to_string(); }
        else if cur.len() + 1 + w.len() <= BIO_WRAP { cur.push(' '); cur.push_str(w); }
        else { lines.push(std::mem::take(&mut cur)); cur = w.to_string(); }
    }
    if !cur.is_empty() { lines.push(cur); }
    lines.truncate(BIO_LINES);
    while lines.len() < BIO_LINES { lines.push(String::new()); }
    let padded: Vec<String> = lines.iter().map(|l| format!("{:<width$}", l, width = BIO_WIDTH)).collect();
    padded.join("\r\n").into_bytes()
}

fn collect(vs3: &mut Disc, bl: &mut Disc, bios: bool, voices: bool) -> io::Result<Vec<(String, Vec<u8>)>> {
    let mut ch: Vec<(String, Vec<u8>)> = vec![];
    let bnr = vs3.find("OPENING.BNR").ok_or_else(|| io::Error::other("OPENING.BNR missing"))?;
    let bd = vs3.data(&bnr)?;
    ch.push((bnr, make_banner(&bd)));
    println!("  New title + banner: Zoids Combo");
    if bios {
        let mut c = 0;
        for n in bio_ids() {
            let (s, d) = (bl.find(&format!("chara/man{:03}.txt", n)), vs3.find(&format!("chara/man{:03}.txt", n)));
            if let (Some(s), Some(d)) = (s, d) {
                let t = bl.data(&s)?;
                let t: String = t.iter().map(|&b| if b < 128 { b as char } else { '\u{FFFD}' }).collect();
                ch.push((d, make_bio(&t))); c += 1;
            }
        }
        println!("  English pilot bios: {}", c);
    }
    if voices {
        let (mut scripts, mut clips) = (0, 0);
        for n in voice_ids() {
            let (ss, sd) = (bl.find(&format!("chara/v_sc/man_{:02}.sc", n)), vs3.find(&format!("chara/v_sc/man{:03}.sc", n)));
            let (Some(ss), Some(sd)) = (ss, sd) else { continue };
            ch.push((sd, bl.data(&ss)?)); scripts += 1;
            let prefix = format!("voice/{:02}/v{:02}_", n, n);
            for p in bl.order.clone() {
                let low = p.to_lowercase();
                if let Some(rest) = low.strip_prefix(&prefix) {
                    if let Some(num) = rest.strip_suffix(".adp") {
                        if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                            let target = format!("voice/{:03}/{:03}_{}.adp", n, n, num);
                            let key = vs3.find(&target).unwrap_or(target);
                            ch.push((key, bl.data(&p)?)); clips += 1;
                        }
                    }
                }
            }
        }
        println!("  English voice scripts: {}, voice clips: {}", scripts, clips);
    }
    #[cfg(feature = "experimental")]
    {
        let tp = vs3.find("etc/demo/text.BIN").ok_or_else(|| io::Error::other("VS3 text.BIN missing"))?;
        let vt = vs3.data(&tp)?;
        let bp = bl.find("etc/demo/TEXT.BIN").ok_or_else(|| io::Error::other("BL TEXT.BIN missing"))?;
        let bt = bl.data(&bp)?;
        let mut gv = |p: &str| -> io::Result<Option<(String, Vec<u8>)>> {
            match vs3.find(p) { Some(k) => { let d = vs3.data(&k)?; Ok(Some((k, d))) } None => Ok(None) }
        };
        let mut gb = |p: &str| -> io::Result<Option<Vec<u8>>> {
            match bl.find(p) { Some(k) => Ok(Some(bl.data(&k)?)), None => Ok(None) }
        };
        let _ = &bt;
        let seed = SEED.load(std::sync::atomic::Ordering::Relaxed);
        let mut ex = gauntlet::build(seed, tp, gauntlet::Inputs { vs3_text: &vt, get_vs3: &mut gv, get_bl: &mut gb })?;
        if let Some((prm_path, prm_data)) = gv("zoid/IDP/zoid_171.prm")? {
            ex.push((prm_path, story::patch_rayse_tiger_prm(&prm_data)?));
        }
        story::patch_dol(&mut vs3.dol)?;
        // a later change to the same file replaces an earlier one
        for (p, d) in ex { ch.retain(|(q, _)| q != &p); ch.push((p, d)); }

        // Battle Legends menu music (point Vs. III's menu screens at its copies of BL's menu songs)
        let bgp = vs3.find("menu/bgm.dat").ok_or_else(|| io::Error::other("menu/bgm.dat missing"))?;
        let bgd = music::apply(&vs3.data(&bgp)?)?;
        ch.retain(|(q, _)| q != &bgp); ch.push((bgp, bgd));
    }
    Ok(ch)
}

fn read_id(p: &Path) -> Option<String> {
    let mut b = [0u8; 6];
    File::open(p).ok()?.read_exact(&mut b).ok()?;
    Some(String::from_utf8_lossy(&b).into_owned())
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (mut vs3p, mut blp, mut outp): (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) = (None, None, None);
    let (mut no_v, mut no_b, mut force) = (false, false, false);
    let mut discs: Vec<PathBuf> = vec![];
    while let Some(a) = args.next() {
        match a.as_str() {
            "--vs3" => vs3p = args.next().map(PathBuf::from),
            "--bl" => blp = args.next().map(PathBuf::from),
            "--out" => outp = args.next().map(PathBuf::from),
            "--no-voices" => no_v = true,
            "--no-bios" => no_b = true,
            "--force" => force = true,
            "--seed" => {
                let v = args.next().unwrap_or_default();
                let n = v.trim().parse::<u64>().map_err(|_| format!("--seed needs a number, got '{}'", v))?;
                #[cfg(feature = "experimental")]
                SEED.store(n, std::sync::atomic::Ordering::Relaxed);
                let _ = n;
            }
            "-h" | "--help" => { println!("usage: zoids_combo_patch [VS3.iso BL.iso] [--out FILE] [--no-voices] [--no-bios] [--force]"); return Ok(()); }
            _ => discs.push(PathBuf::from(a)),
        }
    }
    println!("Zoids Combo Patch Kit v{}", KIT_VERSION);
    if vs3p.is_none() || blp.is_none() {
        let cands: Vec<PathBuf> = if !discs.is_empty() { discs } else {
            let here = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."));
            let mut v: Vec<PathBuf> = std::fs::read_dir(&here).map_err(|e| e.to_string())?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref(), Some("iso" | "gcm" | "rvz")))
                .collect();
            v.sort(); v
        };
        let mut rvz: Vec<String> = vec![];
        for p in cands {
            if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("rvz")).unwrap_or(false) {
                rvz.push(p.file_name().unwrap().to_string_lossy().into_owned()); // only matters if no ISO is found
                continue;
            }
            match read_id(&p).as_deref() {
                Some(VS3_ID) if vs3p.is_none() => vs3p = Some(p),
                Some(BL_ID) if blp.is_none() => blp = Some(p),
                _ => {}
            }
        }
        if (vs3p.is_none() || blp.is_none()) && !rvz.is_empty() {
            return Err(format!("found compressed game(s) {} but not both ISOs. In Dolphin, right-click the game -> Convert File... -> Format: ISO, save it in this folder, then try again.", rvz.join(", ")));
        }
    }
    let (Some(vs3p), Some(blp)) = (vs3p.clone(), blp.clone()) else {
        return Err(format!("need both discs. Put your Zoids Vs. III (T-En) ISO and Battle Legends ISO in this folder\n       (next to this program) and run it again.\n       Found Vs. III: {}   Battle Legends: {}", vs3p.is_some(), blp.is_some()));
    };
    let outp = outp.map(|o| if o.is_absolute() { o } else { vs3p.parent().unwrap_or(Path::new(".")).join(o) })
        .unwrap_or_else(|| vs3p.parent().unwrap_or(Path::new(".")).join("Zoids Combo (Vs3 + Battle Legends).iso"));
    if outp == vs3p || outp == blp { return Err("--out must be a new file, not one of your originals.".into()); }

    let t = std::time::Instant::now();
    let mut vs3 = Disc::open(&vs3p).map_err(|e| format!("can't read {}: {}", vs3p.display(), e))?;
    let mut bl = Disc::open(&blp).map_err(|e| format!("can't read {}: {}", blp.display(), e))?;
    for (d, id, want, label) in [(&mut vs3, VS3_ID, VS3_PRINT, "Zoids Vs. III"), (&mut bl, BL_ID, BL_PRINT, "Battle Legends")] {
        if d.game_id != id { return Err(format!("{} has game ID {}, expected {}. Wrong disc?", label, d.game_id, id)); }
        print!("  checking {} ...", label); io::stdout().flush().ok();
        if fingerprint(d).map_err(|e| e.to_string())? == want { println!(" OK"); } else {
            println!(" DIFFERENT from the reference copy");
            if !force { return Err(format!("your {} is not the exact version this kit expects.\n       Re-run with --force to try anyway (Netplay may desync with friends).", label)); }
            println!("  Continuing because of --force.");
        }
    }
    println!("Collecting changes from your discs:");
    let ch = collect(&mut vs3, &mut bl, !no_b, !no_v).map_err(|e| e.to_string())?;
    println!("Writing {} ...", outp.display());
    build(&mut vs3, &ch, &outp).map_err(|e| e.to_string())?;
    let mut h = Sha1::new();
    let mut f = File::open(&outp).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 1 << 22];
    loop { let k = f.read(&mut buf).map_err(|e| e.to_string())?; if k == 0 { break; } h.update(&buf[..k]); }
    let digest = hex(&h.finalize());
    println!("Done in {}s.  SHA-1: {}", t.elapsed().as_secs(), digest);
    if cfg!(feature = "experimental") {
        println!("TEST BUILD - for testing only. Compare this SHA-1 with your friends before Netplay.");
    } else if !(no_v || no_b || force) {
        if digest == EXPECTED_OUT_SHA1 { println!("Matches the reference build - you are Netplay-ready."); }
        else { println!("WARNING: does not match the reference build. Netplay with friends may desync."); }
    } else { println!("Custom options used: compare this SHA-1 with your friends before Netplay."); }
    Ok(())
}

fn main() {
    let code = match run() { Ok(()) => 0, Err(e) => { println!("ERROR: {}", e); 1 } };
    if std::env::args().len() <= 1 {
        // launched by double-click: keep the window open
        println!("\nPress Enter to close.");
        let _ = io::stdin().lock().read_line(&mut String::new());
    }
    std::process::exit(code);
}
