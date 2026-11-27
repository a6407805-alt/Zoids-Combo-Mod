Test 5: Fixes falling through the stage floor on Jump Melee / Dive attacks (Rayse Tiger & all Zoids), populates stage altitude bounds on ported Battle Legends missions, and includes Battle Legends menu music (Test 4) + Story Mode character portraits & nameplates (Test 2).

- **Floor-clipping fix (`main.dol` + `zoid_171.prm`)**:
  - Hooks `ZoidAllMove` right after `ZoidGround` (`0x8004D5AC`) with `zdClampFloor` (`0x8002AE68`), clamping any live Zoid that tunnels below a walkable `.xb` floor triangle back onto `floor_y` and transitioning descending `ZoidJump` states cleanly into their landing animation (`state 48` + landing dust).
  - Upgrades Rayse Tiger's (`zoid_171.prm`) chest and front-leg ground spheres (`0x00, 0x38, 0x40`) from overlap-only (`1`) to anti-tunneling segment raycast (`2`), matching Blade Liger, Liger Zero, and Whitz Tiger.
- **Stage altitude bounds (`M###.mg` `0x3248..0x3258`)**: sets ceiling (`200`), stage-aware trench floor, and AI flight altitude bounds (`40..120`) instead of zero-filling Vs. III's trailing mission block.
- **Battle Legends menu music (from Test 4)**: points Vs. III's menu screens (`menu/bgm.dat`) at Vs. III's native MusyX copies of Battle Legends' menu songs so title/menus play cleanly.
- **Story Mode portraits & nameplates (from Test 2)**: appends all 75 Battle Legends story portraits (`demosell00.dat` `ModelSet[98..172]`), mirrors horizontal UVs, maps `demoPicMan` + `n_Person.dat` English pilot names, and ports 5 BL opening scenes/missions into slots `002..006`.
