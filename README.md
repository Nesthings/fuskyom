# FUSKYOM — ("fuck streaming keep your own music") 
## cmus-like terminal music player with cover art support 

[![CI](https://github.com/Nesthings/fuskyom/actions/workflows/ci.yml/badge.svg)](https://github.com/Nesthings/fuskyom/actions/workflows/ci.yml)
[![Release](https://github.com/Nesthings/fuskyom/actions/workflows/release.yml/badge.svg)](https://github.com/Nesthings/fuskyom/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A terminal music player inspired by [cmus](https://cmus.github.io/), written
in Rust, that uses [chafa](https://hpjansson.org/chafa/) to draw the album
cover directly in the terminal with colored Unicode/ANSI blocks.

## Visualize album covers embedded in your music

  <img width="1366" height="768" alt="Screenshot_2026-07-08_09-56-31" src="https://github.com/user-attachments/assets/35d0e045-799e-4ac3-8bfa-c1a902100787" />
  <img width="1365" height="692" alt="Screenshot_2026-07-08_09-53-17" src="https://github.com/user-attachments/assets/a58bfec0-a7db-4782-a9f2-a2af8a1468ed" />
  <img width="1365" height="692" alt="Screenshot_2026-07-08_09-54-56" src="https://github.com/user-attachments/assets/0a9b5a41-3067-4db5-9fb8-d4689114fe29" />
  <img width="1365" height="692" alt="Screenshot_2026-07-08_09-55-18" src="https://github.com/user-attachments/assets/344c4b99-516d-45f3-b092-2262f7de3b4f" />


## Explore your library

  <img width="1270" height="767" alt="Screenshot_2026-07-08_13-04-48" src="https://github.com/user-attachments/assets/c0bc342b-1f68-45a0-aea0-a895cd40df18" />
  <img width="956" height="530" alt="Screenshot_2026-07-08_09-59-17" src="https://github.com/user-attachments/assets/a44986b5-f346-43e1-8743-1da3c716539b" />

  
## Type to search feature

<img width="937" height="519" alt="image" src="https://github.com/user-attachments/assets/cbe284b1-ed8f-47f3-a405-ef47d1d203ca" />

  ## What's new

- **Complete Windows support** — See below instructions to install in windows
- **Songs randomizer** — You can switch between album, global or disable the randomization
- **Settings menu** (`9`) — a dedicated panel-based screen for everything
  configurable: default music directories, color themes, and the wave
  visualizer, instead of digging through flags or config files by hand.
- **Multiple default music directories** — configure one or more folders once
  from Settings, and fuskyom launches straight into your library from then
  on, no need to pass a path every time.
  Note: if you initialize the app without passing the path it will choose your
  default system path
- **The browser remembers where you were** — going up/down between
  directories, or hopping over to Now Playing/Settings and back, no longer
  resets you to the top of the list.
- **Wider format support** — M4A, WAV, AIFF/AIF joined MP3/FLAC/OGG.
- **Wave visualizer** — toggle on/off from Settings, switch between Bar Chart
  and Sparkline styles, and it automatically matches whichever color theme is
  active.
- **5 color themes** — Default, Dracula, Gruvbox, Nord, and Neon, switchable
  live from Settings.

<img width="1270" height="757" alt="image" src="https://github.com/user-attachments/assets/5d808e0e-67f2-442a-99ac-3b0417496531" />

<img width="1270" height="757" alt="image" src="https://github.com/user-attachments/assets/6c2da340-a877-4f65-a5a7-36f0107df653" />

<img width="1270" height="757" alt="image" src="https://github.com/user-attachments/assets/52fb33c7-7023-43d5-9eb8-1defc6b34d79" />



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

**Install Rust:**
```powershell
winget install Rustlang.Rustup
```
(or download rustup-init.exe from https://rustup.rs)

**Install chafa:**
```powershell
winget install hpjansson.Chafa
```
alternative: scoop install chafa

**C++ compilation tools (required by Rust linker in Windows)**
```powershell
winget install Microsoft.VisualStudio.2022.BuildTools
```
**during the instalaton, mark the following workload: "Desktop development with C++"**

## Compile and run

```powershell
git clone https://github.com/Nesthings/fuskyom.git
cd fuskyom
cargo build --release
.\target\release\fuskyom.exe "C:\Users\YourUser\Music"
```
**Recomended terminal: Windows Terminal (comes as default in Windows 11, or you can download through Microsoft Store)**
cmd.exe classic and old PowerShell not always renderice correctly Unicode/colors.

## Install on Linux without compiling (latest Release binary)

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

## Keybindings (Browser)

| Key               | Action                                       |
|-------------------|-----------------------------------------------|
| `j` / `↓`         | move selection down                           |
| `k` / `↑`         | move selection up                             |
| `/`               | manual search tool                            |
| `Enter`           | enter folder / play file                      |
| `Backspace`/`h`   | go up a directory (Browser view)              |
| `/`               | open/close filter-by-name search box          |
| `q`               | quit fuskyom                                  |

## Keybindings (player)

| Key               | Action                                       |
|-------------------|-----------------------------------------------|
| `Space`           | pause / resume                                |
| `s`               | stop                                          |
| `n`               | next track in queue                           |
| `p`               | previous track                                |
| `d`               | toggle between randomizer options             |
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


## Keybindings (settings menu)

| Key               | Action                                       |
|-------------------|-----------------------------------------------|
| `j` / `↓`         | move selection down                           |
| `k` / `↑`         | move selection up                             |
| `Enter`           | toggle between options                        |
| `a`               | add new path                                  |
| `d`               | remove path                                   |
| `r`               | refresh player                                |
| `q`               | quit fuskyom                                  |

make to be pointing at the right option at the settings menu


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
