//! Zoids Combo Gauntlet: Mission Mode becomes a roguelike-style tournament ladder.
//! Only compiled with `--features experimental`.
//!
//! - No story: every fight gets a one-line title card, then goes straight to the hangar.
//! - 72 fights from both games, shuffled per run (seed) inside 4 difficulty tiers.
//! - Hangar/shop upgrades between fights are Vs. III's own systems.
//!
//! How routing works (decoded in docs/STORY_PLAN.md): a scene tail `fffc w1 w2 0001 N 9999 ... eeee`
//! sets current mission = N, `ffff` ends the scene and opens the hangar for mission N, and winning
//! mission N plays scene N+1 next. New Game starts at scene 000.
use crate::story::{build_text, join_scene, parse_text, port_mission, split_scene};
use std::io;

pub const FIRST_SLOT: u32 = 1;

/// Vs. III missions used (tutorials 000 and 071-073 left out).
const VS3_MISSIONS: [u32; 30] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 17, 18, 20, 31, 33, 34, 35, 37, 38, 41, 43, 44, 45, 47, 48, 49, 60];
/// Battle Legends missions used (40-43 are near-duplicates of 30-33 and are left out).
const BL_MISSIONS: [u32; 42] = [0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 14, 19, 20, 21, 23, 30, 31, 32, 33, 35, 36, 45, 46, 47, 48, 49, 50, 51, 54, 56, 57, 58, 59, 61, 64, 66, 67, 68, 69, 71];
const TIERS: usize = 4;

#[derive(Clone, Copy, PartialEq)]
enum Src { Vs3, Bl }

struct Fight { src: Src, num: u32, frac: f64 }

/// splitmix64: small deterministic RNG so every friend with the same run code gets the same ladder.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize { (self.next() % n as u64) as usize }
}

/// The ladder for a run: difficulty tiers by how far into its own game each mission was,
/// shuffled inside each tier.
fn ladder(seed: u64) -> Vec<Fight> {
    let mut all: Vec<Fight> = Vec::new();
    for (i, &n) in VS3_MISSIONS.iter().enumerate() { all.push(Fight { src: Src::Vs3, num: n, frac: i as f64 / VS3_MISSIONS.len() as f64 }); }
    for (i, &n) in BL_MISSIONS.iter().enumerate() { all.push(Fight { src: Src::Bl, num: n, frac: i as f64 / BL_MISSIONS.len() as f64 }); }
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for t in 0..TIERS {
        let (lo, hi) = (t as f64 / TIERS as f64, (t + 1) as f64 / TIERS as f64);
        let mut tier: Vec<Fight> = all.iter().filter(|f| f.frac >= lo && f.frac < hi)
            .map(|f| Fight { src: f.src, num: f.num, frac: f.frac }).collect();
        for i in (1..tier.len()).rev() { let j = rng.below(i + 1); tier.swap(i, j); } // Fisher-Yates
        out.extend(tier);
    }
    out
}

fn words(b: &[u8]) -> Vec<u16> { b.chunks(2).filter(|c| c.len() == 2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect() }
fn bytes(w: &[u16]) -> Vec<u8> { w.iter().flat_map(|v| v.to_be_bytes()).collect() }

/// The 8-word per-mission block that follows `ffff` in a scene tail (before `f0f0 ffff`).
fn setup_block(scene: &[u8]) -> [u16; 8] {
    let (_, tail) = split_scene(scene);
    let w = words(&tail);
    let mut out = [0u16; 8];
    if let Some(j) = w.iter().position(|&x| x == 0xf0f0) {
        if j >= 8 { out.copy_from_slice(&w[j - 8..j]); }
    }
    out
}

/// One-line title card + tail that sets current mission = `slot` and opens its hangar.
fn card_scene(text_id: u16, bg: u16, bgm: u16, slot: u16, setup: [u16; 8]) -> Vec<u8> {
    let rec = [0u16, 0, 0, 0, 0, bg, text_id, bgm, 0xffff, 0, 0];
    let mut tail = vec![0xfffc, 0x0001, 0x0047, 0x0001, slot, 0x9999, 0x0001, 0x0002, 0x0002, 0x0002, 0xeeee, 0xffff];
    tail.extend_from_slice(&setup);
    tail.extend_from_slice(&[0xf0f0, 0xffff]);
    join_scene(&[rec], &bytes(&tail))
}

fn title_of(mg: &[u8]) -> String {
    let t: String = mg[0x28..0x8c].iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();
    let t = t.trim().to_string();
    if t.starts_with("MISSION") || !t.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) { String::new() } else { t }
}

fn set_label(mg: &mut [u8], n: usize) {
    let lab = format!("MISSION-{:02}", n);
    let o = 0x2a84;
    for b in mg[o..o + 0x20].iter_mut() { *b = 0; }
    mg[o..o + lab.len()].copy_from_slice(lab.as_bytes());
}

pub struct Inputs<'a> {
    pub vs3_text: &'a [u8],
    pub get_vs3: &'a mut dyn FnMut(&str) -> io::Result<Option<(String, Vec<u8>)>>,
    pub get_bl: &'a mut dyn FnMut(&str) -> io::Result<Option<Vec<u8>>>,
}

pub fn build(seed: u64, vs3_text_path: String, inp: Inputs) -> io::Result<Vec<(String, Vec<u8>)>> {
    let mut text = parse_text(inp.vs3_text);
    let mut ch = Vec::new();
    let fights = ladder(seed);
    let total = fights.len();
    let msg = |text: &mut Vec<Vec<u8>>, s: &str| -> u16 {
        text.push(s.replace('\n', "\r\n").into_bytes());
        (text.len() - 1) as u16
    };

    // The mission file length Vs. III expects.
    let vs3_len = (inp.get_vs3)("menu/map_info_dat/M001.mg")?.map(|(_, d)| d.len()).unwrap_or(13056);
    let mut first_setup = [0u16; 8];
    let mut first_bg = (1u16, 0xffffu16);

    for (i, f) in fights.iter().enumerate() {
        let slot = FIRST_SLOT + i as u32;
        let (scene, mut mg) = match f.src {
            Src::Vs3 => (
                (inp.get_vs3)(&format!("etc/demo/DEMO_{:03}.BIN", f.num))?.map(|(_, d)| d),
                (inp.get_vs3)(&format!("menu/map_info_dat/M{:03}.mg", f.num))?.map(|(_, d)| d),
            ),
            Src::Bl => (
                (inp.get_bl)(&format!("etc/demo/DEMO_{:03}.BIN", f.num))?,
                (inp.get_bl)(&format!("menu/map_info_dat/M{:03}.mg", f.num))?.map(|d| port_mission(&d, vs3_len)),
            ),
        };
        let mg = mg.as_mut().ok_or_else(|| io::Error::other(format!("mission file missing for fight {}", i + 1)))?;
        let scene = scene.unwrap_or_default();
        let setup = setup_block(&scene);
        let (recs, _) = split_scene(&scene);
        let (bg, bgm) = recs.first().map(|r| (r[5], r[7])).unwrap_or((1, 0xffff));
        let title = title_of(mg);
        set_label(mg, i + 1);
        let card = if title.is_empty() { format!("FIGHT {} / {}", i + 1, total) } else { format!("FIGHT {} / {}\n{}", i + 1, total, title) };
        let id = msg(&mut text, &card);
        ch.push((format!("etc/demo/DEMO_{:03}.BIN", slot), card_scene(id, bg, bgm, slot as u16, setup)));
        ch.push((format!("menu/map_info_dat/M{:03}.mg", slot), mg.clone()));
        if i == 0 { first_setup = setup; first_bg = (bg, bgm); }
    }

    // New Game: scene 000 is the Gauntlet intro, then straight to fight 1's hangar.
    let intro = msg(&mut text, &format!("ZOIDS COMBO GAUNTLET\n{} fights. Win to advance, upgrade between fights.", total));
    ch.push(("etc/demo/DEMO_000.BIN".to_string(), card_scene(intro, first_bg.0, first_bg.1, FIRST_SLOT as u16, first_setup)));

    // After the last fight: champion card using Vs. III's own ending tail (sets the game-clear flag).
    let end_slot = FIRST_SLOT + total as u32;
    if let Some((_, end_scene)) = (inp.get_vs3)("etc/demo/DEMO_061.BIN")? {
        let (_, tail) = split_scene(&end_scene);
        // Vs. III's finale sets current mission = 61 before the game-clear flag; point it at the
        // last fight instead so the worst case is a rematch, not a missing mission.
        let mut w = words(&tail);
        for i in 0..w.len().saturating_sub(4) {
            if w[i] == 0xfffc && w[i + 3] == 0x0001 { w[i + 4] = (end_slot - 1) as u16; }
        }
        let tail = bytes(&w);
        let id = msg(&mut text, "GAUNTLET COMPLETE\nYou are the champion of Planet Zi!");
        ch.push((format!("etc/demo/DEMO_{:03}.BIN", end_slot), join_scene(&[[0, 0, 0, 0, 0, first_bg.0, id, 0xffff, 0xffff, 0, 0]], &tail)));
    }

    // Resolve paths to the disc's real names (case-insensitive); new files keep the given name.
    let mut out = Vec::new();
    for (p, d) in ch {
        let real = (inp.get_vs3)(&p)?.map(|(k, _)| k).unwrap_or(p);
        out.push((real, d));
    }
    out.push((vs3_text_path, build_text(&text)));
    println!("  [test] Gauntlet: {} fights (seed {}), story dialogue removed", total, seed);
    Ok(out)
}
