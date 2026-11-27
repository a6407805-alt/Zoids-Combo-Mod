#![allow(dead_code)]
//! Experimental combined Story Mode work. Only compiled with `--features experimental`.
use std::io;

/// VS3 `menu/demo/demosell00.dat` has 98 ModelSets (0..97). BL's 75 unique story portrait
/// sprites (BL `demosell00.dat` root JObj children 0..74) are appended starting at index 98.
pub const VS3_PORTRAIT_BASE: u16 = 98;
pub const BL_PORTRAIT_COUNT: usize = 75;

/// Pilot IDs in `etc/n_Person.dat` (`Tea_persondat`) for BL story portraits 0..74, matching
/// BL's `demoPicMan[0..75]` table so VS3 draws the right English nameplate under each portrait:
///   0..=5  Zan (31)          29..=32 Claudia (36)        50..=57 Kouki (64)
///   6..=10 Tita (32)         33,40,41 Rep. Soldier (43)  58..=65 Reika (65)
///  11..=16 Albane (33)       34..=35 Lezard (38)         66      Rep. Operator (42)
///  17..=22 Max (34)          36..=37 Leviathe (39)       68..=69 Emp. Operator (41)
///  23..=28 Reiner (35)       38,39,67 Emp. Soldier (45)  70..=74 alt Zan/Reiner/Shoma/Kouki/Reika
pub const BL_PORTRAIT_PILOTS: [i32; BL_PORTRAIT_COUNT] = [
    31, 31, 31, 31, 31, 31, //  0..=5:  Zan
    32, 32, 32, 32, 32,     //  6..=10: Tita
    33, 33, 33, 33, 33, 33, // 11..=16: Albane
    34, 34, 34, 34, 34, 34, // 17..=22: Max
    35, 35, 35, 35, 35, 35, // 23..=28: Reiner
    36, 36, 36, 36,         // 29..=32: Claudia
    43,                     // 33:      Republic Soldier A
    38, 38,                 // 34..=35: Lezard
    39, 39,                 // 36..=37: Leviathe
    45, 45,                 // 38..=39: Empire Soldier A
    43, 43,                 // 40..=41: Republic Soldier A
    63, 63, 63, 63, 63, 63, 63, 63, // 42..=49: Shoma
    64, 64, 64, 64, 64, 64, 64, 64, // 50..=57: Kouki
    65, 65, 65, 65, 65, 65, 65, 65, // 58..=65: Reika
    42,                     // 66:      Republic Operator
    45,                     // 67:      Empire Soldier A
    41, 41,                 // 68..=69: Empire Operator
    31, 35, 63, 64, 65,     // 70..=74: Zan, Reiner, Shoma, Kouki, Reika (alt)
];

fn be32(b: &[u8], o: usize) -> u32 { u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }
fn put_be32(b: &mut [u8], o: usize, v: u32) { b[o..o + 4].copy_from_slice(&v.to_be_bytes()); }

/// Map a Battle Legends `charL`/`charR` portrait sprite index to the combined VS3 index.
pub fn remap_portrait(bl_char: u16) -> u16 {
    match bl_char {
        0xFFFF => 0xFFFF,
        0..=74 => VS3_PORTRAIT_BASE + bl_char,
        75..=80 => VS3_PORTRAIT_BASE + 1, // duplicate slots in BL demosell00.dat pointing to BL[1]
        other => other,
    }
}

/// Extract BL's 75 `256x256` `GX_TF_CMPR` story portraits (`0x8000` bytes each) from BL's
/// `menu/demo/demosell00.dat` and append them as `ModelSet[98..172]` to VS3's `demosell00.dat`.
pub fn append_bl_portraits(vs3_dat: &[u8], bl_dat: &[u8]) -> io::Result<Vec<u8>> {
    if vs3_dat.len() < 0x360 || bl_dat.len() < 0x100 {
        return Err(io::Error::other("demosell00.dat too small"));
    }
    let (v_dsz, v_rc) = (be32(vs3_dat, 4) as usize, be32(vs3_dat, 8) as usize);
    let rel_off = 0x20 + v_dsz;
    let tail_off = rel_off + v_rc * 4;
    if tail_off + 8 > vs3_dat.len() {
        return Err(io::Error::other("VS3 demosell00.dat header out of bounds"));
    }

    // Walk BL scene_data -> models[0] -> root JObj -> 75 child JObjs to extract CMPR textures.
    let (b_dsz, b_rc) = (be32(bl_dat, 4) as usize, be32(bl_dat, 8) as usize);
    let b_pub = be32(bl_dat, 0x20 + b_dsz + b_rc * 4) as usize;
    let b_models = be32(bl_dat, 0x20 + b_pub) as usize;
    let b_mset0 = be32(bl_dat, 0x20 + b_models) as usize;
    let b_root_jobj = be32(bl_dat, 0x20 + b_mset0) as usize;
    let mut b_child = be32(bl_dat, 0x20 + b_root_jobj + 8) as usize;
    let mut bl_imgs: Vec<&[u8]> = Vec::with_capacity(BL_PORTRAIT_COUNT);
    for _ in 0..BL_PORTRAIT_COUNT {
        if b_child == 0 { return Err(io::Error::other("BL demosell00.dat missing expected child JObj")); }
        let dobj = be32(bl_dat, 0x20 + b_child + 0x10) as usize;
        let mobj = be32(bl_dat, 0x20 + dobj + 0x08) as usize;
        let tobj = be32(bl_dat, 0x20 + mobj + 0x08) as usize;
        let imdesc = be32(bl_dat, 0x20 + tobj + 0x4C) as usize;
        let img_off = be32(bl_dat, 0x20 + imdesc) as usize;
        let start = 0x20 + img_off;
        if start + 0x8000 > bl_dat.len() {
            return Err(io::Error::other("BL portrait texture out of bounds"));
        }
        bl_imgs.push(&bl_dat[start..start + 0x8000]);
        b_child = be32(bl_dat, 0x20 + b_child + 0x0C) as usize;
    }

    let mut data = Vec::with_capacity(v_dsz + BL_PORTRAIT_COUNT * 0x8360);
    data.extend_from_slice(&vs3_dat[0x20..0x20 + v_dsz]);
    let mut relocs = Vec::with_capacity(v_rc + BL_PORTRAIT_COUNT * 35 + 200);
    let mut tmpl_relocs = Vec::new();
    for i in 0..v_rc {
        let r = be32(vs3_dat, rel_off + i * 4);
        relocs.push(r);
        if r < 0x320 && r != 0x06C { tmpl_relocs.push(r as usize); }
    }

    // Copy VS3 ModelSet[0]'s 0x320-byte descriptor block and mirror U horizontally:
    // BL portraits face right in texture space while VS3 portraits face left (VS3 flips left-side
    // portraits with scale.x = -1 and leaves right-side at +1, opposite of BL).
    let mut tmpl = [0u8; 0x320];
    tmpl.copy_from_slice(&vs3_dat[0x20..0x340]);
    let (u0, u3) = (be32(&tmpl, 0x40), be32(&tmpl, 0x58));
    put_be32(&mut tmpl, 0x40, u3);
    put_be32(&mut tmpl, 0x58, u0);
    let (u1, u2) = (be32(&tmpl, 0x48), be32(&tmpl, 0x50));
    put_be32(&mut tmpl, 0x48, u2);
    put_be32(&mut tmpl, 0x50, u1);

    // GX_TF_CMPR textures require 32-byte alignment in RAM (archive header is 0x20 bytes).
    while data.len() & 0x1F != 0 { data.push(0); }
    let mut img_offs = Vec::with_capacity(BL_PORTRAIT_COUNT);
    for img in &bl_imgs {
        img_offs.push(data.len() as u32);
        data.extend_from_slice(img);
    }

    let mut new_msets = Vec::with_capacity(BL_PORTRAIT_COUNT);
    for &img_off in &img_offs {
        let desc_off = data.len() as u32;
        let mut blk = tmpl;
        put_be32(&mut blk, 0x06C, img_off);
        relocs.push(desc_off + 0x06C);
        for &tr in &tmpl_relocs {
            let v = be32(&blk, tr) + desc_off;
            put_be32(&mut blk, tr, v);
            relocs.push(desc_off + tr as u32);
        }
        data.extend_from_slice(&blk);

        let ma_off = data.len() as u32;
        for (k, d) in [0x224u32, 0x26C, 0x2B4, 0x2FC].into_iter().enumerate() {
            data.extend_from_slice(&(desc_off + d).to_be_bytes());
            relocs.push(ma_off + (k as u32) * 4);
        }
        data.extend_from_slice(&0u32.to_be_bytes());

        let ms_off = data.len() as u32;
        data.extend_from_slice(&(desc_off + 0x1A8).to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&ma_off.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        relocs.push(ms_off);
        relocs.push(ms_off + 8);
        new_msets.push(ms_off);
    }

    // Build expanded null-terminated models array (98 VS3 + 75 BL + null) and repoint scene_data.
    let pub_root = be32(vs3_dat, tail_off) as usize;
    let old_models = be32(&data, pub_root) as usize;
    let mut vs3_count = 0usize;
    while be32(&data, old_models + vs3_count * 4) != 0 { vs3_count += 1; }
    if vs3_count != VS3_PORTRAIT_BASE as usize {
        return Err(io::Error::other("unexpected VS3 demosell00 ModelSet count"));
    }
    let new_models = data.len() as u32;
    for i in 0..vs3_count {
        let m = be32(&data, old_models + i * 4);
        data.extend_from_slice(&m.to_be_bytes());
        relocs.push(new_models + (i as u32) * 4);
    }
    for (i, &ms) in new_msets.iter().enumerate() {
        data.extend_from_slice(&ms.to_be_bytes());
        relocs.push(new_models + ((vs3_count + i) as u32) * 4);
    }
    data.extend_from_slice(&0u32.to_be_bytes());
    put_be32(&mut data, pub_root, new_models);

    let tail = &vs3_dat[tail_off..];
    let out_len = 0x20 + data.len() + relocs.len() * 4 + tail.len();
    let mut out = Vec::with_capacity(out_len);
    out.extend_from_slice(&vs3_dat[..0x20]);
    put_be32(&mut out, 0, out_len as u32);
    put_be32(&mut out, 4, data.len() as u32);
    put_be32(&mut out, 8, relocs.len() as u32);
    out.extend_from_slice(&data);
    for r in relocs { out.extend_from_slice(&r.to_be_bytes()); }
    out.extend_from_slice(tail);
    Ok(out)
}

/// Copy BL's concise English pilot names into VS3's `etc/n_Person.dat` (104-byte records, 32-byte
/// NUL-terminated name at offset 0) for war-era pilots so portrait nameplates fit cleanly.
pub fn patch_person_names(vs3_person: &[u8], bl_person: &[u8]) -> Vec<u8> {
    let mut out = vs3_person.to_vec();
    for id in (31usize..=46).chain(63..=67) {
        let o = id * 104;
        if o + 32 <= out.len() && o + 32 <= bl_person.len() {
            let src = &bl_person[o..o + 32];
            let end = src.iter().position(|&b| b == 0).unwrap_or(32);
            if end > 0 && src[..end].iter().all(|&b| b.is_ascii()) {
                out[o..o + 32].fill(0);
                out[o..o + end].copy_from_slice(&src[..end]);
            }
        }
    }
    out
}

fn va_to_dol(dol: &[u8], va: u32, len: usize) -> io::Result<usize> {
    for i in 0..18 {
        let (fo, sa, sz) = (be32(dol, i * 4), be32(dol, 0x48 + i * 4), be32(dol, 0x90 + i * 4));
        if fo != 0 && va >= sa && va.saturating_add(len as u32) <= sa.saturating_add(sz) {
            let off = (fo + (va - sa)) as usize;
            if off + len <= dol.len() { return Ok(off); }
        }
    }
    Err(io::Error::other(format!("VA 0x{:08X} out of bounds in VS3 DOL", va)))
}

fn ppc_b(from_va: u32, to_va: u32, lk: bool) -> u32 {
    let disp = (to_va as i64 - from_va as i64) as i32;
    0x4800_0000 | ((disp as u32) & 0x03FF_FFFC) | u32::from(lk)
}

fn ppc_bc(op_bo_bi: u32, from_idx: usize, to_idx: usize) -> u32 {
    let disp = ((to_idx as isize - from_idx as isize) * 4) as i16;
    op_bo_bi | ((disp as u16 as u32) & 0xFFFC)
}

/// Assemble `zdClampFloor(Zoid* r3)` at `cave_va` (`0x8002AE68`).
/// Runs at the end of `ZoidAllMove` right after `ZoidGround` to prevent high-speed dive attacks
/// (such as Rayse Tiger's Jump Melee Dive, `anim 65/66`) from tunneling below single-sided `.xb`
/// floor triangles and falling forever.
fn build_zd_clamp_floor(cave_va: u32) -> Vec<u32> {
    const ZD_COL_POS_Y: u32 = 0x8003_0434;
    const ZD_SET_XYZ: u32 = 0x8003_0FC0;
    const S_MAKE_MATRIX: u32 = 0x8003_6E78;
    const ZD_UPDATE_BONES: u32 = 0x8003_2D08;
    const BEQ: u32 = 0x4182_0000;
    const BNE: u32 = 0x4082_0000;
    const BGE: u32 = 0x4080_0000; // branch if >= (and on NaN unordered, since FL == 0)

    let mut c = Vec::with_capacity(68);
    let mut fix_exit = Vec::new();
    let mut fix_upd = Vec::new();

    c.extend_from_slice(&[
        0x7C08_02A6, // mflr r0
        0x9001_0004, // stw  r0, 0x04(r1)
        0x9421_FFE0, // stwu r1, -0x20(r1)
        0x93E1_001C, // stw  r31, 0x1C(r1)
        0x7C7F_1B78, // mr   r31, r3
        0x281F_0000, // cmplwi r31, 0
    ]);
    fix_exit.push((c.len(), BEQ)); c.push(0);
    c.push(0x801F_0040); // lwz r0, 0x40(r31) (root JObj)
    c.push(0x2800_0000); // cmplwi r0, 0
    fix_exit.push((c.len(), BEQ)); c.push(0);

    // Exempt intentional sub-floor states: 42 (ZoidActDead), 43 (ZoidActDeath), 51 (zdModeGene)
    c.push(0x801F_1DA8); // lwz r0, 0x1DA8(r31) (action)
    for act in [42u16, 43, 51] {
        c.push(0x2C00_0000 | act as u32); // cmpwi r0, act
        fix_exit.push((c.len(), BEQ)); c.push(0);
    }

    // Query walkable floor triangle closest in Y to zoid->pos (0x1C18)
    c.extend_from_slice(&[
        0x3800_FFFF, // li   r0, -1
        0xB001_0008, // sth  r0, 0x08(r1) (polyIdx = 0xFFFF)
        0x387F_1C18, // addi r3, r31, 0x1C18 (&zoid->pos)
        0x3881_0008, // addi r4, r1, 0x08 (&polyIdx)
    ]);
    let call_col_va = cave_va + (c.len() as u32) * 4;
    c.push(ppc_b(call_col_va, ZD_COL_POS_Y, true));
    c.push(0xA001_0008); // lhz r0, 0x08(r1)
    c.push(0x2800_FFFF); // cmplwi r0, 0xFFFF
    fix_exit.push((c.len(), BEQ)); c.push(0);

    // Compare zoid->pos.y (f0) against floor_y (f1)
    c.push(0xC01F_1C1C); // lfs f0, 0x1C1C(r31)
    c.push(0xFC00_0840); // fcmpo cr0, f0, f1
    fix_exit.push((c.len(), BGE)); c.push(0);

    // Below floor: clamp pos.y & ground_y to floor_y, mark grounded, clamp negative vertical speeds
    c.extend_from_slice(&[
        0xD03F_1C1C, // stfs f1, 0x1C1C(r31) (pos.y = floor_y)
        0xD03F_1BF0, // stfs f1, 0x1BF0(r31) (ground_y = floor_y)
        0x3800_0001, // li   r0, 1
        0x901F_1C08, // stw  r0, 0x1C08(r31) (on_ground = 1)
        0xC042_86B8, // lfs  f2, -0x7948(r2) (0.0f @ 0x80389C78)
        0xC01F_1D34, // lfs  f0, 0x1D34(r31) (vy_atk)
        0xFC00_1040, // fcmpo cr0, f0, f2
        0x4080_0008, // bge  +8
        0xD05F_1D34, // stfs f2, 0x1D34(r31)
        0xC01F_1D00, // lfs  f0, 0x1D00(r31) (vy_move)
        0xFC00_1040, // fcmpo cr0, f0, f2
        0x4080_0008, // bge  +8
        0xD05F_1D00, // stfs f2, 0x1D00(r31)
    ]);

    // If descending in ZoidJump (action 1 or 2, substate < 48, vy_jump < 0), trigger landing (48)
    c.extend_from_slice(&[
        0x801F_1DA8, // lwz   r0, 0x1DA8(r31)
        0x2C00_0001, // cmpwi r0, 1
        0x4182_000C, // beq   +12 (chk_jmp)
        0x2C00_0002, // cmpwi r0, 2
    ]);
    fix_upd.push((c.len(), BNE)); c.push(0);
    // chk_jmp:
    c.push(0x801F_1DC0); // lwz   r0, 0x1DC0(r31) (jump substate)
    c.push(0x2C00_0030); // cmpwi r0, 48
    fix_upd.push((c.len(), BGE)); c.push(0);
    c.push(0xC01F_1DCC); // lfs   f0, 0x1DCC(r31) (vy_jump)
    c.push(0xFC00_1040); // fcmpo cr0, f0, f2
    fix_upd.push((c.len(), BGE)); c.push(0);
    c.extend_from_slice(&[
        0x3800_0030, // li  r0, 48 (ZoidJump landing state)
        0x901F_1DC0, // stw r0, 0x1DC0(r31)
        0x3800_003C, // li  r0, 60 (landing timer)
        0x901F_1DD4, // stw r0, 0x1DD4(r31)
    ]);

    let upd_idx = c.len();
    c.push(0x7FE3_FB78); // mr r3, r31
    let call_xyz_va = cave_va + (c.len() as u32) * 4;
    c.push(ppc_b(call_xyz_va, ZD_SET_XYZ, true));
    c.push(0x807F_0040); // lwz r3, 0x40(r31)
    let call_mtx_va = cave_va + (c.len() as u32) * 4;
    c.push(ppc_b(call_mtx_va, S_MAKE_MATRIX, true));
    c.push(0x7FE3_FB78); // mr r3, r31
    let call_bones_va = cave_va + (c.len() as u32) * 4;
    c.push(ppc_b(call_bones_va, ZD_UPDATE_BONES, true));

    let exit_idx = c.len();
    c.extend_from_slice(&[
        0x83E1_001C, // lwz  r31, 0x1C(r1)
        0x3821_0020, // addi r1, r1, 0x20
        0x8001_0004, // lwz  r0, 0x04(r1)
        0x7C08_03A6, // mtlr r0
        0x4E80_0020, // blr
    ]);

    for (idx, cond) in fix_upd { c[idx] = ppc_bc(cond, idx, upd_idx); }
    for (idx, cond) in fix_exit { c[idx] = ppc_bc(cond, idx, exit_idx); }
    c
}

/// Patch VS3 `main.dol`:
/// 1. Populate `demoPicMan[98..173]` (`0x8014BD14`) so appended BL portraits show pilot nameplates.
/// 2. Install `zdClampFloor` in unused devkit routine `TEA_usbmccControlChk` (`0x8002AE60..0x8002B00C`)
///    and hook the `dummyFunc` call after `ZoidGround` in `ZoidAllMove` (`0x8004D5AC..0x8004D5B0`).
pub fn patch_dol(vs3_dol: &mut [u8]) -> io::Result<()> {
    const DEMO_PIC_MAN_VA: u32 = 0x8014_BD14;
    let base = va_to_dol(vs3_dol, DEMO_PIC_MAN_VA, (VS3_PORTRAIT_BASE as usize + BL_PORTRAIT_COUNT) * 4)?;
    let start = base + (VS3_PORTRAIT_BASE as usize) * 4;
    for (i, &pilot) in BL_PORTRAIT_PILOTS.iter().enumerate() {
        let o = start + i * 4;
        vs3_dol[o..o + 4].copy_from_slice(&pilot.to_be_bytes());
    }

    const USB_STUB_VA: u32 = 0x8002_AE60;
    const CAVE_VA: u32 = 0x8002_AE68;
    const USB_MAX_BYTES: usize = 0x1AC; // 0x8002AE60..0x8002B00C
    const HOOK_VA: u32 = 0x8004_D5AC;

    let cave_code = build_zd_clamp_floor(CAVE_VA);
    let total_cave_bytes = 8 + cave_code.len() * 4;
    if total_cave_bytes > USB_MAX_BYTES {
        return Err(io::Error::other("zdClampFloor exceeds TEA_usbmccControlChk size"));
    }
    let stub_off = va_to_dol(vs3_dol, USB_STUB_VA, total_cave_bytes)?;
    let hook_off = va_to_dol(vs3_dol, HOOK_VA, 8)?;
    if be32(vs3_dol, stub_off) != 0x7C08_02A6
        || be32(vs3_dol, hook_off) != 0x3862_8D94
        || be32(vs3_dol, hook_off + 4) != 0x4BFC_C6AD
    {
        return Err(io::Error::other("unexpected VS3 DOL instructions at floor-clamp hook site"));
    }

    // Stub TEA_usbmccControlChk entry with `li r3, 0; blr`, then write zdClampFloor at +8
    put_be32(vs3_dol, stub_off, 0x3860_0000);
    put_be32(vs3_dol, stub_off + 4, 0x4E80_0020);
    for (i, &w) in cave_code.iter().enumerate() {
        put_be32(vs3_dol, stub_off + 8 + i * 4, w);
    }

    // Replace `addi r3, r2, -0x726C; bl dummyFunc` after `bl ZoidGround` with `mr r3, r31; bl zdClampFloor`
    put_be32(vs3_dol, hook_off, 0x7FE3_FB78);
    put_be32(vs3_dol, hook_off + 4, ppc_b(HOOK_VA + 4, CAVE_VA, true));
    Ok(())
}

/// Upgrade Rayse Tiger (`zoid_171.prm`) chest (`0x00`) and front-leg (`0x38`, `0x40`) ground
/// collision spheres from mode `1` (overlap-only) to mode `2` (segment raycast), matching
/// Blade Liger (`zoid_013`), Liger Zero (`zoid_021`), and Whitz Tiger (`zoid_179`).
pub fn patch_rayse_tiger_prm(prm: &[u8]) -> io::Result<Vec<u8>> {
    if prm.len() < 0x100 {
        return Err(io::Error::other("zoid_171.prm too small"));
    }
    let mut out = prm.to_vec();
    for off in [0x00usize, 0x38, 0x40] {
        if be32(&out, off) == 1 {
            put_be32(&mut out, off, 2);
        }
    }
    Ok(out)
}

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

/// Copy one BL scene's records into a VS3 scene slot: remap BL portraits to their appended VS3
/// ModelSet indices, append and renumber BL messages in `text`, and keep the VS3 slot's own tail.
pub fn port_scene(bl_scene: &[u8], bl_text: &[Vec<u8>], vs3_slot: &[u8], text: &mut Vec<Vec<u8>>) -> Vec<u8> {
    let (mut recs, _) = split_scene(bl_scene);
    let (_, vs3_tail) = split_scene(vs3_slot);
    let mut map = std::collections::HashMap::new();
    for r in recs.iter_mut() {
        r[1] = remap_portrait(r[1]);
        r[3] = remap_portrait(r[3]);
        let m = r[6];
        if m >= 0xfff0 || (m as usize) >= bl_text.len() { continue; }
        let new = *map.entry(m).or_insert_with(|| { text.push(bl_text[m as usize].clone()); (text.len() - 1) as u16 });
        r[6] = new;
    }
    join_scene(&recs, &vs3_tail)
}

/// Convert a BL mission file (`0x321C` bytes) to VS3's `0x3300`-byte layout, populating the
/// Vs. III stage height/flight bounds block (`0x3248..0x3258`) so knockback ceilings and AI
/// flying Zoids have proper altitude limits instead of zero.
pub fn port_mission(bl: &[u8], vs3_len: usize) -> Vec<u8> {
    let mut d = bl.to_vec();
    d.resize(vs3_len.max(bl.len()), 0);
    if d.len() >= 0x3258 {
        let mp_end = d[..16.min(d.len())].iter().position(|&b| b == 0).unwrap_or(0);
        let map_name = std::str::from_utf8(&d[..mp_end]).unwrap_or("");
        let limit_l: i32 = match map_name {
            "map_p22" => -240,
            "map_p29" => -100,
            "map_p18" | "map_p03" => -50,
            _ => -15,
        };
        d[0x3248..0x324C].copy_from_slice(&200i32.to_be_bytes());     // limitHighH (ceiling)
        d[0x324C..0x3250].copy_from_slice(&limit_l.to_be_bytes());    // limitHighL (trench floor)
        d[0x3250..0x3254].copy_from_slice(&120i32.to_be_bytes());     // AI flight max altitude
        d[0x3254..0x3258].copy_from_slice(&40i32.to_be_bytes());      // AI flight min altitude
    }
    d
}

pub struct Inputs<'a> {
    pub vs3_text: &'a [u8],
    pub bl_text: &'a [u8],
    pub get_vs3: &'a mut dyn FnMut(&str) -> io::Result<Option<(String, Vec<u8>)>>,
    pub get_bl: &'a mut dyn FnMut(&str) -> io::Result<Option<Vec<u8>>>,
}

/// Test 5: BL story portraits + nameplates, Rayse Tiger / global floor-clipping fix, and 5 BL
/// opening scenes + missions in slots `002..006`.
pub fn experiment(vs3_text_path: String, inp: Inputs) -> io::Result<Vec<(String, Vec<u8>)>> {
    let mut text = parse_text(inp.vs3_text);
    let bl_text = parse_text(inp.bl_text);
    let before = text.len();
    let mut ch = Vec::new();

    let (ds_path, vs3_ds) = (inp.get_vs3)("menu/demo/demosell00.dat")?
        .ok_or_else(|| io::Error::other("VS3 demosell00.dat missing"))?;
    let bl_ds = (inp.get_bl)("menu/demo/demosell00.dat")?
        .ok_or_else(|| io::Error::other("BL demosell00.dat missing"))?;
    ch.push((ds_path, append_bl_portraits(&vs3_ds, &bl_ds)?));

    if let (Some((pp, vp)), Some(bp)) = ((inp.get_vs3)("etc/n_Person.dat")?, (inp.get_bl)("etc/n_Person.dat")?) {
        ch.push((pp, patch_person_names(&vp, &bp)));
    }
    if let Some((prm_path, prm_data)) = (inp.get_vs3)("zoid/IDP/zoid_171.prm")? {
        ch.push((prm_path, patch_rayse_tiger_prm(&prm_data)?));
    }

    // (BL scene/mission, VS3 slot): linear 002 -> 003 -> 004 -> 005 -> 006 sequence
    let pairs: [(u32, u32); 5] = [
        (0, 2),  // "Who is it?"               - Republic Op, Shoma, Kouki
        (8, 3),  // "Earth is Burning"         - Zan, Tita, Shoma, Republic Op
        (21, 4), // "Hit it on the Head"       - Reiner, Kouki, Empire Op
        (9, 5),  // "Giant Shadow"             - Reika, Leviathe, Empire Soldier, Zan, Tita, Shoma
        (12, 6), // "Suspicion of the Crimson" - Albane, Tita, Shoma
    ];
    for (bl_n, slot) in pairs {
        let bl_scene = (inp.get_bl)(&format!("etc/demo/DEMO_{:03}.BIN", bl_n))?.ok_or_else(|| io::Error::other("BL scene missing"))?;
        let (slot_path, slot_data) = (inp.get_vs3)(&format!("etc/demo/DEMO_{:03}.BIN", slot))?.ok_or_else(|| io::Error::other("VS3 scene missing"))?;
        ch.push((slot_path, port_scene(&bl_scene, &bl_text, &slot_data, &mut text)));
        if let (Some(blm), Some((mp, md))) = ((inp.get_bl)(&format!("menu/map_info_dat/M{:03}.mg", bl_n))?,
                                              (inp.get_vs3)(&format!("menu/map_info_dat/M{:03}.mg", slot))?) {
            ch.push((mp, port_mission(&blm, md.len())));
        }
    }
    println!(
        "  [test] Story: {} BL portraits appended, {} scenes ported (slots 002..006), {} dialogue lines added",
        BL_PORTRAIT_COUNT, pairs.len(), text.len() - before
    );
    ch.push((vs3_text_path, build_text(&text)));
    Ok(ch)
}
