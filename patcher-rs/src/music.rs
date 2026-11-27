//! Battle Legends menu music in Zoids Combo. Only compiled with `--features experimental`.
//!
//! Each menu screen's music comes from `menu/bgm.dat` (a track number per screen). The track
//! number indexes a table in main.dol: entry = 0x8013ac60 + 0x6490 + n * 0x2c, holding
//! (u16 sound group, u16 song id, name). Vs. III's menu tracks (bgm05x) are menu-only, so we
//! copy Battle Legends' menu tracks (all MusyX files, from the player's own BL disc) over them
//! and patch the group/song ids to Battle Legends' values.
use std::io;

const TABLE: u32 = 0x8013ac60 + 0x6490;
const EXTS: [&str; 6] = ["son", "h", "poo", "pro", "sam", "sdi"];

/// (VS3 track number, VS3 file name, BL file name, BL group id, BL song id)
/// VS3 menu screens -> track: 0 title=48, 1-2=51, 3=52, 4,5,9,10=53, 6=49.
/// BL menu screens  -> track: 0=bgm00c, 1=bgm019, 3=bgm02d, 4=bgm003, 6=bgm008.
const SWAPS: [(u32, &str, &str, u16, u16); 5] = [
    (48, "bgm050", "bgm00c", 40, 31), // title / main menu
    (51, "bgm053", "bgm019", 17, 8),
    (52, "bgm054", "bgm02d", 14, 5),
    (53, "bgm055", "bgm003", 31, 22),
    (49, "bgm051", "bgm008", 35, 26),
];

fn dol_offset(dol: &[u8], addr: u32) -> Option<usize> {
    let be = |o: usize| u32::from_be_bytes([dol[o], dol[o + 1], dol[o + 2], dol[o + 3]]);
    for i in 0..18 {
        let (off, a, sz) = (be(i * 4), be(0x48 + i * 4), be(0x90 + i * 4));
        if sz != 0 && a <= addr && addr < a + sz { return Some((off + addr - a) as usize); }
    }
    None
}

/// Returns file changes; patches `dol` in place.
pub fn apply(
    dol: &mut Vec<u8>,
    get_vs3: &mut dyn FnMut(&str) -> Option<String>,
    get_bl: &mut dyn FnMut(&str) -> io::Result<Option<Vec<u8>>>,
) -> io::Result<Vec<(String, Vec<u8>)>> {
    let mut ch = Vec::new();
    for (n, vs3_name, bl_name, grp, song) in SWAPS {
        // only patch if the table entry really is the track we expect (protects against other builds)
        let Some(o) = dol_offset(dol, TABLE + n * 0x2c) else { continue };
        if &dol[o + 4..o + 4 + vs3_name.len()] != vs3_name.as_bytes() { continue; }
        let mut files = Vec::new();
        for ext in EXTS {
            let (Some(src), Some(dst)) = ((get_bl)(&format!("audio/bgm/{}.{}", bl_name, ext))?,
                                          (get_vs3)(&format!("audio/bgm/{}.{}", vs3_name, ext))) else { break };
            files.push((dst, src));
        }
        if files.len() != EXTS.len() { continue; }
        dol[o..o + 2].copy_from_slice(&grp.to_be_bytes());
        dol[o + 2..o + 4].copy_from_slice(&song.to_be_bytes());
        ch.extend(files);
    }
    println!("  [test] Battle Legends menu music: {} tracks", ch.len() / EXTS.len());
    Ok(ch)
}
