# Soin 8-Bit SFX Player

Tiny retro terminal player for browsing and playing the OGG files in this repository.

## Run

From this directory:

```bash
cargo run
```

If you run the binary from another location, pass the audio directory explicitly:

```bash
cargo run -- --audio-root ../assets/audio/sfx
```

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
