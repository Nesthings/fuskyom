mod app;
mod art;
mod library;
mod player;
mod ui;

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event};

use app::App;

fn main() -> anyhow::Result<()> {
    let start_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(dirs::audio_dir)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));

    if !start_dir.is_dir() {
        eprintln!(
            "'{}' is not a directory. Use: fuskyom [your-music-dir]",
            start_dir.display()
        );
        std::process::exit(1);
    }

    silence_stderr_to_log();

    let (event_tx, event_rx) = mpsc::channel();
    let cmd_tx = player::spawn(event_tx);
    let mut app = App::new(start_dir, cmd_tx.clone());

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &mut app, &event_rx);
    ratatui::restore();

    let _ = cmd_tx.send(player::PlayerCommand::Shutdown);
    result
}

/// ALSA (and occasionally other native audio libs) write warnings like
/// "underrun occurred" straight to fd 2, bypassing our TUI entirely and
/// corrupting the screen. Those messages are harmless noise from the audio
/// backend, not errors from this app, so we redirect stderr to a log file
/// instead of letting them splatter over the alternate screen. Anything
/// genuinely worth showing to the user goes through PlayerEvent::Error and
/// is rendered in the status line instead.
fn silence_stderr_to_log() {
    use std::os::unix::io::AsRawFd;

    let log_path = std::env::temp_dir().join("fuskyom-stderr.log");
    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        unsafe {
            libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO);
        }
        // Leak the handle on purpose: fd 2 now points at it for the whole
        // process lifetime, closing our end here would be wrong.
        std::mem::forget(file);
    }
}

fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    event_rx: &mpsc::Receiver<player::PlayerEvent>,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Drain any pending events from the audio thread first so the UI
        // reflects the latest playback state before we redraw.
        while let Ok(event) = event_rx.try_recv() {
            app.on_player_event(event);
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
