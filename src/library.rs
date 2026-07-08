use std::path::{Path, PathBuf};

/// A single entry in a directory listing: either a subdirectory or an audio file.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "oga"];

/// Returns true if the given path has an extension we know how to play.
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Reads a directory and returns its entries sorted: directories first, then
/// audio files, both alphabetically (case-insensitive). Non-audio, non-dir
/// files are skipped entirely so the browser only ever shows things you can
/// act on.
pub fn read_dir_sorted(dir: &Path) -> anyhow::Result<Vec<Entry>> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for item in std::fs::read_dir(dir)? {
        let item = match item {
            Ok(i) => i,
            Err(_) => continue,
        };
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_string();

        // Skip hidden dotfiles/dirs, cmus doesn't show them either by default.
        if name.starts_with('.') {
            continue;
        }

        let is_dir = path.is_dir();
        if is_dir {
            dirs.push(Entry {
                path,
                name,
                is_dir: true,
            });
        } else if is_audio_file(&path) {
            files.push(Entry {
                path,
                name,
                is_dir: false,
            });
        }
    }

    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    dirs.extend(files);
    Ok(dirs)
}

/// Returns just the audio files (no directories) in a directory, sorted, used
/// to build a play queue when the user hits Enter on a track: everything from
/// that track onward in the same folder gets queued up.
pub fn audio_files_in(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    Ok(read_dir_sorted(dir)?
        .into_iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.path)
        .collect())
}
