# Handoff: Zoids Combo combined Story Mode

You are continuing work on **Zoids Combo**, a friends-only fan mod for Dolphin that combines the GameCube games **Zoids Vs. III** (Japan, `GZVJDA`, with the T-En v1.00 English fan translation by Traceytrace) and **Zoids: Battle Legends** (USA, `GZSE70`). Repo: `github.com/a6407805-alt/Zoids-Combo-Mod`. Read `README.md` and `docs/STORY_PLAN.md` first.

## Hard rules
- **Never commit or distribute game data.** The patcher builds everything at patch time from the player's own ISOs. No ISOs, extracted files, textures or audio in the repo or releases.
- **Never enable AI credit overages in Antigravity.** Use all available models and switch models when one runs low on quota.
- Keep the stable build unchanged unless deliberately releasing. Experimental work goes behind the `experimental` Cargo feature in `patcher-rs`.

## How the project works
- `patcher-rs/` (Rust) reads both ISOs, applies changes, writes `Zoids Combo.iso` with game ID `GZVJZC`. `src/main.rs` is the patcher; `src/story.rs` is experimental story code (only built with `--features experimental`).
- `patcher-py/` is an older Python version of the stable patcher (byte-identical stable output). It does not have the story work.
- `launcher/` (C#/WPF, .NET Framework 4.8) auto-updates from GitHub Releases, builds the game, launches Dolphin. It has a "Get test builds" checkbox. When it's ticked, the launcher uses the newest `test-N` pre-release.
- **Stable release:** bump `VERSION` (for example `1.2.2`), update `CHANGES.md`, push to `main`. `.github/workflows/release.yml` builds and publishes.
- **Test release:** bump `TESTVERSION` (next is `2`), update `TESTNOTES.md`, push to `main`. `.github/workflows/test-release.yml` publishes pre-release `test-N` with the experimental patcher.
- Pushing git tags is not needed; releases are driven by those files.
- The stable patcher's expected output SHA-1 is `9c1161f6961798018433d8ad56f7f515d46d9f7e`. If you change stable output, update `EXPECTED_OUT_SHA1` in both patchers.

## Goal
**One Story Mode ("Mission Mode" in the menu), one save, no side select and no A/B choices.** Play the Battle Legends campaign first (war era), then a time skip into the Vs. III campaign. Difficulty rises steadily across the whole run. Treat it as its own story: merge near-duplicate missions, keep unique battles, use best judgement. The chapter order is in `docs/STORY_PLAN.md`.

## What is known (see docs/STORY_PLAN.md for detail)
- Story scenes: `etc/demo/DEMO_###.BIN`. 22-byte records `[index, charL, faceL, charR, faceR, bg, msg, bgm, -1, 0, 0]`, then a tail of route/flag records (`fffc ... 9999 ... eeee` = route marker, `fffe ... 7777 ... cccc` = choice, `fffd ... 8888 ... dddd` = alternate). The tails are NOT simple next-scene pointers; game code (`ttl_demoflagset`, `calcEndMission`, `menufncMission_Menu`) decides what plays next.
- Text: `etc/demo/text.BIN` (VS3, 4000 real entries; the header claims more but stray text follows the table) and BL `etc/demo/TEXT.BIN` (2071 entries). u32 offset table then strings. Appending BL messages after VS3's works.
- Missions: `menu/map_info_dat/M###.mg`, one per mission, numbered like the scenes. BL files are 12828 bytes and VS3 files 13056; all fields sit at the same offsets, so zero-padding BL files to 13056 works.
- BL scenes run 0-72 (68 with content), BL missions are 46 files. VS3 scenes run 0-74 (58 with content), VS3 missions are 34 files.
- Both discs include an unstripped `zoidR.elf` with ~8000 symbols for reverse-engineering. VS3's code is BL's plus about 200 functions.
- The engine is HAL's HSD/Sysdolphin (same as Smash Melee), so Melee tools like HSDRaw may open `.dat` files.

## Test 1 result (passed)
Test build 1 put BL scene 000 + mission M000 into VS3 slot 002. Route in game: Mission Mode > New Game > Normal > advance the opening > at "Take tutorial / Don't take tutorial" choose **Don't take tutorial** > slot 002 plays. BL English dialogue, BL backgrounds, the BL Map Info briefing ("Who is it?", Atak Kat Noir) and the battle on BL's stage all worked.

**Bug found:** character portraits are wrong. BL's `charL`/`charR` numbers point to different VS3 characters.

## Next tasks, in order
1. **Portrait remap.** Work out which portrait numbers mean which character in each game (portrait/character data lives under `chara/` and the scene scripts). Build a BL -> VS3 table in `story.rs`. Where VS3 has no matching character, copy BL's portrait files in under unused numbers. Ship as test build 2 and verify Shoma, Tita, Zan, Kouki, Reiner and others look right.
2. **Add all BL scenes and missions under new numbers** (for example 100+), not replacing VS3 ones. First confirm the engine accepts scene and mission numbers above 74: check array sizes and save-data mission flags (`ttlMain_MissionFlagChkSet`, `MemCard*` functions).
3. **Decode next-scene logic** (`ttl_demoflagset`, `calcEndMission`, `menufncMission_Menu`) so you can rewrite tails: remove all choices, chain BL scenes in the planned order, link BL's final scene into VS3's opening.
4. **Difficulty curve:** scale enemy settings per mission. VS3 also has a Normal/Easy select at New Game.
5. When stable: fold the story work into the default build and remove the launcher's separate "Battle Legends Campaign" button.

## Testing
You cannot run the game yourself. Publish a test build, then have the user (or a computer-use agent on their PC) open the launcher with "Get test builds" ticked, start the game, and check. Dolphin on the user's PC: player 1 is a PS5 controller (profile "PS5"). For keyboard testing, set Port 1 to keyboard with Default (A = X, B = Z, Start = Enter, stick = arrows), then reload the "PS5" profile afterwards. Keys need to be held around 0.1 s to register. Write each result into the test log in `docs/STORY_PLAN.md`.

## Build commands
- Patcher (Linux cross-compile): `cd patcher-rs && cargo build --release --target x86_64-pc-windows-gnu [--features experimental]` (needs mingw-w64). On Windows just `cargo build --release`.
- Launcher: `cd launcher && dotnet build -c Release`.
