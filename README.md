# fuskyom — ("fuck streaming keep your own music") 
## cmus-like terminal music player with cover art support 

[![CI](https://github.com/Nesthings/fuskyom/actions/workflows/ci.yml/badge.svg)](https://github.com/Nesthings/fuskyom/actions/workflows/ci.yml)
[![Release](https://github.com/Nesthings/fuskyom/actions/workflows/release.yml/badge.svg)](https://github.com/Nesthings/fuskyom/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A terminal music player inspired by [cmus](https://cmus.github.io/), written
in Rust, that uses [chafa](https://hpjansson.org/chafa/) to draw the album
cover directly in the terminal with colored Unicode/ANSI blocks.

## Architecture

Just like cmus, audio playback runs on its **own thread**, completely
separate from the UI thread (`src/player.rs`). They communicate over channels
(`std::sync::mpsc`), never through shared memory directly — so audio never
glitches because of a slow render (rendering with chafa can take a few
milliseconds).

- `src/player.rs` — audio thread: opens the device with `rodio`, decodes
  with `symphonia`/`claxon`/`lewton` depending on the format, exposes
  Play/Pause/Stop/Seek/Volume through commands.
- `src/library.rs` — directory browser (only folders and audio files, like
  cmus's browser).
- `src/art.rs` — extracts the embedded cover art from the file with `lofty`,
  pipes it to `chafa` as a subprocess via stdin, and caches the result (ANSI
  → ratatui `Text` via `ansi-to-tui`) so chafa isn't re-invoked on every
  frame.
- `src/app.rs` — application state and key handling.
- `src/ui.rs` — draws both views with `ratatui`.

## Requirements

### Linux (any distro)

1. **Install Rust via rustup** — this step is the same regardless of distro,
   and it's the recommended way on all of them, not just Ubuntu:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```
2. **Verify it's actually the rustup toolchain being picked up**, not a
   leftover system one:
   ```bash
   which cargo   # must print something under $HOME/.cargo/bin, not /usr/bin
   cargo --version
   ```
   On Ubuntu/Debian-based distros specifically, `apt`'s `rustc`/`cargo`
   packages ship an old toolchain (`1.75`) that's too old for some
   dependencies here (`~1.85+` needed) and can shadow rustup's. If `which
   cargo` still points at `/usr/bin/cargo`, remove the distro package so
   there's no ambiguity:
   ```bash
   sudo apt remove --purge cargo rustc
   hash -r
   which cargo
   ```
   Arch and Fedora's `rustc`/`cargo` packages track upstream releases
   closely enough that this usually isn't an issue, but the check above is
   still worth running once.
3. **Install chafa and the ALSA dev headers** (chafa renders the album art
   at runtime; the ALSA headers are needed at build time to compile the
   audio backend):

   | Distro                  | Command                                              |
   |--------------------------|------------------------------------------------------|
   | Debian / Ubuntu / Pop!_OS | `sudo apt install chafa libasound2-dev pkg-config`   |
   | Arch / Manjaro           | `sudo pacman -S chafa alsa-lib pkgconf`               |
   | Fedora                  | `sudo dnf install chafa alsa-lib-devel pkgconf-pkg-config` |

### macOS

1. **Install Rust via rustup**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```
2. **Install chafa and Xcode command line tools** (the latter provides the
   linker/build tools cargo needs) via [Homebrew](https://brew.sh):
   ```bash
   xcode-select --install
   brew install chafa
   ```

### Windows

Not officially tested. `rodio`/`cpal` support WASAPI, so audio should work,
but you'll need `chafa` available on your PATH (e.g. via
[scoop](https://scoop.sh) or building it from source) — chafa doesn't
publish official Windows binaries.


## Install without compiling (latest Release binary)

Every time a `vX.Y.Z` tag is published, GitHub Actions builds ready-to-run
binaries for Linux and macOS and uploads them to the repo's **Releases**
section.

```bash
# Linux x86_64
curl -L -o fuskyom.tar.gz \
  https://github.com/Nesthings/fuskyom/releases/latest/download/fuskyom-linux-x86_64.tar.gz
tar xzf fuskyom.tar.gz
sudo apt install chafa   # runtime dependency, not bundled in
./fuskyom-linux-x86_64/fuskyom [Your music path]
```

## Build from source

```bash
cargo build --release
./target/release/fuskyom [Your music path]
# or with no argument: it tries your default music folder, falling back to $HOME
```

Supported formats: **MP3, FLAC, OGG/Vorbis**.

## Keybindings

| Key               | Action                                       |
|-------------------|-----------------------------------------------|
| `j` / `↓`         | move selection down                           |
| `k` / `↑`         | move selection up                             |
| `/`               | manual search tool                            |
| `Enter`           | enter folder / play file                      |
| `Backspace`/`h`   | go up a directory (Browser view)              |
| `Space`           | pause / resume                                |
| `s`               | stop                                          |
| `n`               | next track in queue                           |
| `p`               | previous track                                |
| `+` / `-`         | volume up / down                              |
| `,` / `.`         | seek back / forward 5s                        |
| `r`               | toggle repeat queue                           |
| `/`               | open/close filter-by-name search box          |
| `1` / `2` / `Tab` | Browser / Now Playing / toggle between views  |
| `q`               | quit fuskyom                                  |

Pressing `/` opens a search box above the file list; typing filters entries
by name live, `↑`/`↓` still move between matches, `Enter` plays the
highlighted result and closes the box, and pressing `/` again (or `Esc`)
closes it and restores the full list — so typed characters never leak into
playback commands while you're filtering.

When you play a file, `fuskyom` builds the queue from every audio file in
that same folder starting at the track you picked — just like cmus, so
`n`/`p` walk through the whole album.

## Publishing a new Release

The `.github/workflows/release.yml` workflow only triggers on tags shaped
like `vX.Y.Z`:

```bash
git tag v0.1.0
git push origin v0.1.0
```

That kicks off the GitHub Actions build for Linux and macOS, packages each
binary together with the README and LICENSE, and publishes them
automatically to the repo's **Releases** section (with release notes
generated from the commit history).

## Development

On every `push`/pull request to `main`, `.github/workflows/ci.yml` runs
formatting (`cargo fmt --check`), lints (`cargo clippy -D warnings`), build,
and tests — so any PR that breaks something gets caught before merging.

```bash
cargo fmt              # format
cargo clippy            # check lints locally before pushing
cargo check             # fast compile check, no final binary
```

## Notes on chafa

If a file has no embedded cover art, the "Now Playing" view says so instead
of showing art. If `chafa` isn't installed, that's also reported right there
instead of failing silently.

The art size is recalculated (and chafa re-invoked) if you resize the
terminal; the result is cached per (track, width, height) so it isn't
re-rendered on every UI frame.
