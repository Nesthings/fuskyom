use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ansi_to_tui::IntoText;
use ratatui::text::Text;

/// Caches rendered album art per (file, width, height) so we don't shell out
/// to chafa on every single redraw -- only when the track or pane size
/// changes. `None` means "we tried and there's no art / chafa isn't
/// available", cached too so we don't keep retrying every frame.
pub struct ArtRenderer {
    cache: HashMap<(PathBuf, u16, u16), Option<Text<'static>>>,
    pub chafa_missing: bool,
}

impl ArtRenderer {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            chafa_missing: false,
        }
    }

    /// Renders (or returns cached) album art for `path` sized to fit a
    /// `width`x`height` character cell area.
    pub fn render(&mut self, path: &Path, width: u16, height: u16) -> Option<Text<'static>> {
        if width == 0 || height == 0 {
            return None;
        }
        let key = (path.to_path_buf(), width, height);
        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }

        let result = self.render_uncached(path, width, height);
        self.cache.insert(key, result.clone());
        result
    }

    fn render_uncached(&mut self, path: &Path, width: u16, height: u16) -> Option<Text<'static>> {
        let bytes = extract_cover_bytes(path)?;
        match run_chafa(&bytes, width, height) {
            Ok(ansi) => ansi.into_text().ok(),
            Err(_) => {
                self.chafa_missing = true;
                None
            }
        }
    }
}

/// Pulls the first embedded picture (front cover, or whatever's first) out of
/// a track's tags using lofty. Returns the raw encoded image bytes (jpeg/png).
fn extract_cover_bytes(path: &Path) -> Option<Vec<u8>> {
    use lofty::file::TaggedFileExt;
    let tagged = lofty::probe::Probe::open(path).ok()?.read().ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    let picture = tag.pictures().first()?;
    Some(picture.data().to_vec())
}

/// Pipes image bytes into `chafa` and captures its ANSI/Unicode block output.
fn run_chafa(bytes: &[u8], width: u16, height: u16) -> anyhow::Result<String> {
    let size_arg = format!("{width}x{height}");
    let mut child = Command::new("chafa")
        .args([
            "--size", &size_arg, "--format", "symbols", "--colors", "full", "--polite", "on", "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("not found 'chafa' in PATH: {e}"))?;

    if let Some(stdin) = child.stdin.take() {
        let mut stdin = stdin;
        let _ = stdin.write_all(bytes);
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("chafa error");
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
