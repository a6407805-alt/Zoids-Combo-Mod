//! Experimental combined Story Mode work. Only compiled with `--features experimental`.
use std::io;

/// etc/demo/text.BIN: u32 offset table (count = first offset / 4), then strings back to back.
pub fn parse_text(b: &[u8]) -> Vec<Vec<u8>> {
    let be = |o: usize| u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as usize;
    // The fan translation's table claims more entries than it has (stray text follows the
    // real table), so stop at the first offset that is out of range or goes backwards.
    let claimed = be(0) / 4;
    let mut offs: Vec<usize> = Vec::new();
    for i in 0..claimed {
        let o = be(i * 4);
        if o > b.len() || offs.last().is_some_and(|&p| o < p) { break; }
        offs.push(o);
    }
    let n = offs.len();
    offs.push(b.len());
    (0..n).map(|i| b[offs[i]..offs[i + 1].max(offs[i])].to_vec()).collect()
}

pub fn build_text(msgs: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = msgs.len() * 4;
    for m in msgs { out.extend_from_slice(&(pos as u32).to_be_bytes()); pos += m.len(); }
    for m in msgs { out.extend_from_slice(m); }
    out
}

/// DEMO_###.BIN: 22-byte records [index, charL, faceL, charR, faceR, bg, msg, bgm, -1, 0, 0], then a tail.
pub fn split_scene(b: &[u8]) -> (Vec<[u16; 11]>, Vec<u8>) {
    let mut recs = Vec::new();
    let mut i = 0;
    while i + 22 <= b.len() {
        let mut r = [0u16; 11];
        for k in 0..11 { r[k] = u16::from_be_bytes([b[i + 2 * k], b[i + 2 * k + 1]]); }
        if r[0] as usize != recs.len() { break; }
        recs.push(r);
        i += 22;
    }
    (recs, b[i..].to_vec())
}

pub fn join_scene(recs: &[[u16; 11]], tail: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for r in recs { for v in r { out.extend_from_slice(&v.to_be_bytes()); } }
    out.extend_from_slice(tail);
    out
}

/// Copy one BL scene's records into a VS3 scene slot. Its messages are appended to `text`
/// and renumbered; the VS3 slot keeps its own tail (route/flag markers) so the story flow is unchanged.
pub fn port_scene(bl_scene: &[u8], bl_text: &[Vec<u8>], vs3_slot: &[u8], text: &mut Vec<Vec<u8>>) -> Vec<u8> {
    let (mut recs, _) = split_scene(bl_scene);
    let (_, vs3_tail) = split_scene(vs3_slot);
    let mut map = std::collections::HashMap::new();
    for r in recs.iter_mut() {
        let m = r[6];
        if m >= 0xfff0 || (m as usize) >= bl_text.len() { continue; }
        let new = *map.entry(m).or_insert_with(|| { text.push(bl_text[m as usize].clone()); (text.len() - 1) as u16 });
        r[6] = new;
    }
    join_scene(&recs, &vs3_tail)
}

/// BL mission files (12828 bytes) share VS3's layout (13056 bytes) except for a short trailing block.
pub fn port_mission(bl: &[u8], vs3_len: usize) -> Vec<u8> {
    let mut d = bl.to_vec();
    d.resize(vs3_len.max(bl.len()), 0);
    d
}

pub struct Inputs<'a> {
    pub vs3_text: &'a [u8],
    pub bl_text: &'a [u8],
    pub get_vs3: &'a mut dyn FnMut(&str) -> io::Result<Option<(String, Vec<u8>)>>,
    pub get_bl: &'a mut dyn FnMut(&str) -> io::Result<Option<Vec<u8>>>,
}

/// Test 1: Vs. III's first story mission (scene/mission 002) becomes Battle Legends' first (000).
pub fn experiment(vs3_text_path: String, inp: Inputs) -> io::Result<Vec<(String, Vec<u8>)>> {
    let mut text = parse_text(inp.vs3_text);
    let bl_text = parse_text(inp.bl_text);
    let before = text.len();
    let mut ch = Vec::new();
    let pairs: [(u32, u32); 1] = [(0, 2)]; // (BL scene, VS3 slot)
    for (bl_n, slot) in pairs {
        let bl_scene = (inp.get_bl)(&format!("etc/demo/DEMO_{:03}.BIN", bl_n))?.ok_or_else(|| io::Error::other("BL scene missing"))?;
        let (slot_path, slot_data) = (inp.get_vs3)(&format!("etc/demo/DEMO_{:03}.BIN", slot))?.ok_or_else(|| io::Error::other("VS3 scene missing"))?;
        ch.push((slot_path, port_scene(&bl_scene, &bl_text, &slot_data, &mut text)));
        if let (Some(blm), Some((mp, md))) = ((inp.get_bl)(&format!("menu/map_info_dat/M{:03}.mg", bl_n))?,
                                              (inp.get_vs3)(&format!("menu/map_info_dat/M{:03}.mg", slot))?) {
            ch.push((mp, port_mission(&blm, md.len())));
        }
    }
    println!("  [test] Story: BL scene/mission 000 -> VS3 slot 002, {} lines of BL dialogue added", text.len() - before);
    ch.push((vs3_text_path, build_text(&text)));
    Ok(ch)
}
