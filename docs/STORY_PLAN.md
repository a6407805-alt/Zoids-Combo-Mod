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

## Script format notes
- `etc/demo/DEMO_###.BIN`: 22-byte records `[index, charL, faceL, charR, faceR, bg, msg, bgm, -1, 0, 0]`, then a tail.
- Tail `fffc ...` = continue; `fffe a A ... 7777 b B ... cccc` = choice (A -> scene A, B -> scene B); `fffd ... 8888 ... dddd` = alternate branch.
- `etc/demo/TEXT.BIN`: u32 offset table (count = first offset / 4), then strings back to back.
- BL's English scripts are already on the VS3 disc in `etc/demo_US/`.
- Battle settings (stage, enemies, win condition) are NOT in the scripts: still to be mapped.
