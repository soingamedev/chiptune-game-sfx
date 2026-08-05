# Soin GAME-Bit SFX/MUSIC Player

Tiny retro terminal player for browsing and playing audio in this repository.

## Formats

- **OGG / WAV** — decoded by rodio.
- **Tracker modules: XM / MOD / S3M / IT** — rendered by
  [`xmrsplayer`](https://crates.io/crates/xmrsplayer) (same `xmrs` lineage the
  Game Boy Advance `agb_tracker` uses, so a `.xm` previewed here matches the
  on-device music). Tracker tracks loop until you stop them.

The library scans a root of `category/…` subfolders (the 404-OGG pack) **and**
audio files sitting directly in the root (a "flat" folder), so you can point it
at any folder of sounds.

## Run

From this directory:

```bash
cargo run
```

Point it at another folder with `--audio-root` — e.g. audition a GBA project's
generated music/SFX (its `.xm` + `.wav`):

```bash
cargo run -- --audio-root ../assets/audio/sfx
cargo run -- --audio-root "<path>"
```

Pointed at the GBA project's `audio/` folder you'll see two categories: `music`
(the `.xm` themes) and `sfx` (the `.wav` effects).

## Controls

| Key | Action |
| --- | --- |
| `Up` / `Down` | Move in the active list |
| `Left` / `Right` | Switch between categories and sounds |
| `Tab` | Toggle active panel |
| `Enter` | Play selected sound |
| `Space` | Pause/resume, or play selected sound if stopped |
| `s` | Stop |
| `/` | Search sounds |
| `Esc` | Leave search, or quit when not searching |
| `+` / `-` | Adjust volume |
| `q` | Quit |

## Build

Windows:

```bash
cargo build --release
```

Linux:

```bash
cargo build --release
```

On Linux, make sure your system has a working default audio device. Rodio uses the platform audio backend available on the host.
