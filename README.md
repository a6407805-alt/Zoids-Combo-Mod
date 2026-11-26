# Zoids Combo: Vs. III + Battle Legends

A friends-only fan mod that combines **Zoids Vs. III** (with the T-En English fan translation by Traceytrace) and **Zoids: Battle Legends** for Dolphin, playable online with Dolphin NetPlay.

**This repository contains no game data.** The tools build the combined game from your own, legally owned copies of both games.

## For players

1. Download **ZoidsComboLauncher.exe** from the latest release (no install needed; runs on Windows 10/11).
2. Have both games as ISO files (in Dolphin: right-click a game → Convert File… → Format: ISO):
   - Zoids Vs. III (Japan) `GZVJDA`, with T-En v1.00 applied
   - Zoids: Battle Legends (USA) `GZSE70`
3. Open the launcher. It finds your ISOs and Dolphin, downloads the latest patch, builds the game, and adds it to Dolphin's game list.
4. Press **PLAY**. To play together, press **Play online…** in the launcher for NetPlay and Hamachi steps.

The launcher checks for updates every time it opens, and rebuilds the game automatically when there's a new patch.

## What's in it so far

- Its own title and banner in Dolphin ("Zoids Combo", game ID `GZVJZC`)
- English pilot bios and battle voices from Battle Legends
- Coming: the Battle Legends campaign as a separate mode, alongside Vs. III's own story

## For developers

| Folder | What |
|---|---|
| `patcher-rs/` | The patcher (Rust). Released as `ZoidsComboPatcher.exe`. |
| `patcher-py/` | The same patcher in Python, byte-identical output (Mac/Linux). |
| `launcher/` | The launcher (C#/WPF, .NET Framework 4.8). |

To release: update `CHANGES.md`, then push a tag such as `v1.3.0`. GitHub Actions builds both programs and publishes the release, and every launcher picks it up on next start.
