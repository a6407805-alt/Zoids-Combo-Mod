# Combined Story Mode plan

One Story Mode, one save, no side select and no branching choices. Battle Legends' war era is played first, then a time skip ("A long time has passed...") into Zoids Vs. III.

## Rules
- All A/B choices removed in both games. Take whichever route has the better or more varied battles.
- Unique battles from dropped routes are kept as extra missions; near-duplicate missions are cut.
- Enemy difficulty rises steadily from the first mission to the last.

## Order (scene numbers are DEMO_###.BIN in each game)
| # | Chapter | Source scenes |
|---|---|---|
| 1 | The War Begins | BL 0-3 |
| 2 | Rottiger | BL 19-21 |
| 3 | Uneasy Allies | BL 4-8 (+ rival battle from BL 14-18) |
| 4 | Bloody Death Saurer | BL 9-12 |
| 5 | Rottiger's Missions | BL 37, 45-51 |
| 6 | United Front | BL 30-33 |
| 7 | Mad Thunder | BL 34-36 |
| 8 | The Hidden Base | BL 52-59 |
| 9 | Final Battle (war era) | BL 60-62 |
| - | Time skip | - |
| 10 | The Crimson Gale | VS3 1-3, 71-73 |
| 11 | Dark Forces | VS3 4-6 |
| 12 | Mina | VS3 7-13 |
| 13 | On the Run | VS3 31-36, 41-47 |
| 14 | Mega Saurer | VS3 37-38, 48-50, 60-61 |

## Script & portrait format notes
- `etc/demo/DEMO_###.BIN`: 22-byte records `[index, charL, faceL, charR, faceR, bg, msg, bgm, -1, 0, 0]`, then a tail.
  - `charL` / `charR` (`0xFFFF` = `-1` skips): portrait expression sprite index in `menu/demo/demosell00.dat` (NOT `chara/bp/`, which is battle pilot portraits).
  - `faceL` / `faceR` (`0..5`): animation state (`0` hidden, `1` slide in, `2` stay visible, `3` slide out, `4` fade out, `5` fade in). Both games map `0..5 -> matanim [3, 2, 0, 1, 1, 2]`. Note `char = 0` is a real portrait when `face != 0` (`BL[0]` = Zan, `VS3[0]` = Arrow).
  - VS3 `demosell00.dat` has 98 `ModelSet`s (`0..97`, all VS3-era characters, facing left in raw texture space; VS3 sets `scale.x = -1` on Left and `+1` on Right). BL `demosell00.dat` has 1 `ModelSet` with 81 child `JObj`s (`0..74` are 75 unique `256x256` `GX_TF_CMPR` war-era portraits facing right in raw texture space; BL sets `scale.x = +1` on Left and `-1` on Right).
  - Appending `BL[0..74]` as `ModelSet[98..172]` with horizontally mirrored UVs, plus populating `demoPicMan[98..172]` (`0x8014BD14` in `main.dol`, 395 `i32` slots) and `etc/n_Person.dat` names, renders BL portraits and their English nameplates natively in VS3.
- `etc/demo/TEXT.BIN`: u32 offset table (count = first offset / 4), then strings back to back.
- BL's English scripts are already on the VS3 disc in `etc/demo_US/`.
- Battle settings live in `menu/map_info_dat/M###.mg`, one file per mission, numbered like the story scenes.
  Fields seen: map picture (`map_pNN`), title, win/lose condition text, stage (`mNN`), area name and briefing,
  enemy Zoid IDs (`szNNN`) with names and weapons, and a mission label (`MISSION-01`).
  BL files are 12828 bytes (`0x321C`), VS3 files 13056 (`0x3300`). All text/unit fields sit at identical offsets in both
  (unit blocks every 0x24c from 0x5c4, mission label at 0x2a84). In VS3's extra `0x321C..0x3300` trailer, `0x3248..0x3258`
  holds stage altitude limits (`limitHighH` ceiling, `limitHighL` floor, AI flight max/min Y), which `port_mission` populates per stage.
- **Engine floor-clipping bug (`zdClampFloor` in `test-5`)**: in vanilla VS3, Rayse Tiger (`zoid_171.prm`) has `mode = 0` on its paw spheres and `mode = 1` (overlap-only) on its chest/front legs, and `ZoidGround`'s anti-tunneling segment check at `0x80042FC0` has an empty `if` body. Nose-first dive attacks (`ZoidJump` `anim 65/66`) could cross single-sided `.xb` floor triangles in one frame and fall forever. `patch_dol` installs `zdClampFloor` at `0x8002AE68` (hooked at `0x8004D5AC` after `ZoidGround`) and upgrades `zoid_171.prm` front spheres to `mode = 2`.
- BL has 46 mission files, VS3 has 34.

## Scene tails & progression (decoded from `zoidR.elf`)
In `menu_MissionDemoScrPlay1` (`0x8008D3F0`), `ttl_demoflagset` (`0x8008C280`), and `ttlMain` (`0x80063D2C`):
- `ffff ...` (`-1`): ends `menu_MissionDemo`, returning to Mission Menu / Map Info for `current_mission_no` (`0x8015A938`).
- `fffe msgA nextDemoA flagIdxA flagValA 7777 msgB nextDemoB flagIdxB flagValB cccc` (`-2`): 2-way choice; calls `ttl_demoflagset(flagIdx, flagVal)` and, if `nextDemo != -1`, immediately loads `DEMO_{nextDemo:03}.BIN` inside the demo player.
- `fffd 0000 nextDemo flagIdx flagVal 8888 ... dddd` (`-3`): conditional jump; if `flag[flagIdx] == flagVal` and `nextDemo != -1`, immediately loads `DEMO_{nextDemo:03}.BIN`.
- `fffc w1 w2 flagIdx flagVal 9999 ... eeee` (`-4`): calls `ttl_demoflagset(flagIdx, flagVal)`. When `flagIdx == 1` and `flagVal < 100`, sets `current_mission_no` (`0x8015A938`) = `flagVal`!
- After winning a mission battle, `ttlMain` (`0x80063D30..0x80063D54`) increments `current_mission_no` by `+1` and plays `DEMO_{current_mission_no:03}.BIN` next. (Note `ttl_demoflagset` checks `< 100` at `0x8008C2E0` / `0x8008C358` — keep in mind for Task 2 when numbering scenes above 74!)

## Build steps
1. Convert BL `M###.mg` to the VS3 layout.
2. Merge BL story text into VS3 `text.BIN` (append and renumber message IDs).
3. Renumber BL scenes and missions into an unused range (e.g. 100+) so they don't clash with VS3's 000-074.
4. Rewrite scene tails: remove choices, chain BL's last scene into VS3's first.
5. First playable test: Story Mode starts at BL mission 1 inside VS3.
6. Difficulty curve via enemy settings per mission.

## Test log
### Test 1 (test-1): BL scene/mission 000 in VS3 slot 002 - PASSED
- Path: Mission Mode > New Game > Normal > opening scene > "Don't take tutorial" > slot 002.
- BL English dialogue displays correctly from the merged text.BIN (messages 4000+). No text limit hit.
- BL backgrounds display correctly (same background numbering).
- Map Info shows BL mission: "Who is it?", Empire Development Base, BL map, win/lose conditions, enemy Atak Kat Noir with model + weapons.
- Battle starts on BL's stage with a 10:00 timer and the enemy on radar.
- PROBLEM: character portraits are wrong (BL charL/charR numbers point at different VS3 characters).
  Needs a BL -> VS3 portrait remap table (and BL portrait files copied in where VS3 lacks them).
- Note: VS3 has a Normal/Easy difficulty select at New Game.

## Menu music (test 3)
- `menu/bgm.dat`: 50 x s16, track number per menu screen. VS3: 48,51,51,52,53,53,49,42,43,53,53. BL: 12,25,24,41,3,7,8,42,43.
- Track table in main.dol: entry = 0x8013ac60 + 0x6490 + n*0x2c = (u16 group, u16 song, name). VS3 DOL file offset for track 48 is 0x13e930.
- Same-named tracks differ between the games (only .sam matches), so swaps copy all 6 MusyX files and patch group/song ids.
- Test 3 copied BL bgm00c/019/02d/003/008 files over VS3 bgm050/053/054/055/051 and patched ids: music played garbled (BL files are an older MusyX export).
- Test 4 (current): VS3 already has its own re-exports of the same songs (same numbers/names, matching ids), so `music.rs` only writes BL's menu numbers into VS3's `menu/bgm.dat`.

## Direction change: Gauntlet (test 6)
The user dropped the combined story in favour of a roguelike-style tournament: no dialogue, every mission from both games in one run (72 fights), shuffled per run inside 4 difficulty tiers, upgrades between fights, no permadeath.
- `patcher-rs/src/gauntlet.rs` builds it (experimental): slots 001-072, scene 000 = intro card, scene 073 = champion card using VS3's finale tail.
- Each fight scene: 1 card record + `fffc 0001 0047 0001 <slot> 9999 0001 0002 0002 0002 eeee ffff <8-word setup block from the mission's original scene> f0f0 ffff`.
- Run seed: patcher `--seed N`; launcher stores it as `seed` and shows "GAUNTLET RUN CODE". Same seed = byte-identical ISO.
- The 8-word setup block after `ffff` is still not fully decoded (looks like pilot/zoid/partner + reward data).
