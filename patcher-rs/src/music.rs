//! Battle Legends menu music in Zoids Combo. Only compiled with `--features experimental`.
//!
//! `menu/bgm.dat` holds one s16 track number per menu screen. Vs. III already contains its own
//! re-exported copies of every Battle Legends menu song (same track numbers and names, Vs. III's
//! newer MusyX format), so we only point Vs. III's menu screens at those tracks.
//! (Copying Battle Legends' own files over Vs. III's tracks plays garbled: different MusyX version.)
use std::io;

/// Battle Legends' menu track per screen (from BL's menu/bgm.dat).
/// Screens 9 and 10 only exist in Vs. III; they reuse BL's screen 4 and 5 music.
const BL_MENU: [(usize, i16); 9] = [(0, 12), (1, 25), (2, 24), (3, 41), (4, 3), (5, 7), (6, 8), (9, 3), (10, 7)];

pub fn apply(vs3_bgm_dat: &[u8]) -> io::Result<Vec<u8>> {
    let mut d = vs3_bgm_dat.to_vec();
    if d.len() < 22 { return Err(io::Error::other("menu/bgm.dat too small")); }
    for (screen, track) in BL_MENU {
        d[screen * 2..screen * 2 + 2].copy_from_slice(&track.to_be_bytes());
    }
    println!("  [test] Battle Legends menu music: {} menu screens", BL_MENU.len());
    Ok(d)
}
