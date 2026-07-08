# Contributing to fuskyom

Thanks for your interest. The flow is the typical GitHub one:

1. Fork the repo and create a descriptive branch (`git checkout -b fix/something`).
2. Before opening your PR, run locally the same checks CI runs:
   ```bash
   cargo fmt
   cargo clippy --all-targets -- -D warnings
   cargo build --release
   cargo test
   ```
3. Open the Pull Request against `main`. The CI workflow
   (`.github/workflows/ci.yml`) runs automatically and must pass before
   merging.

## Open ideas

- `.m3u` playlist support.
- A spectrum visualizer alongside the album art.
- More audio formats (AAC, WAV, MP4) — `rodio`/`symphonia` already support
  them, it's just a matter of adding them to `AUDIO_EXTENSIONS` in
  `src/library.rs` and to the `rodio` feature flags in `Cargo.toml`.
- switch color themes 

If you're adding a new dependency, prioritize crates with active maintenance
and a reasonable MSRV — this project is built in CI against the latest
`stable` toolchain via `dtolnay/rust-toolchain`.
