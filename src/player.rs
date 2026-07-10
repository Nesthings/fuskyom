use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

/// Commands the UI thread sends to the audio thread.
pub enum PlayerCommand {
    Play(PathBuf),
    TogglePause,
    Stop,
    SetVolume(f32),
    SeekBy(f32),
    Shutdown,
}

/// Events the audio thread reports back to the UI thread.
pub enum PlayerEvent {
    Started {
        path: PathBuf,
        duration: Option<Duration>,
    },
    Position(Duration),
    Paused(bool),
    Finished,
    Error(String),
}

fn read_duration(path: &Path) -> Option<Duration> {
    use lofty::file::AudioFile;
    let tagged = lofty::probe::Probe::open(path).ok()?.read().ok()?;
    Some(tagged.properties().duration())
}

pub fn spawn(event_tx: Sender<PlayerEvent>) -> Sender<PlayerCommand> {
    let (cmd_tx, cmd_rx): (Sender<PlayerCommand>, Receiver<PlayerCommand>) =
        std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let (_stream, stream_handle): (OutputStream, OutputStreamHandle) =
            match OutputStream::try_default() {
                Ok(s) => s,
                Err(e) => {
                    let _ =
                        event_tx.send(PlayerEvent::Error(format!("cannot open audio device: {e}")));
                    return;
                }
            };

        let sink = match Sink::try_new(&stream_handle) {
            Ok(s) => s,
            Err(e) => {
                let _ = event_tx.send(PlayerEvent::Error(format!("cannot create audio sink: {e}")));
                return;
            }
        };

        let mut has_track = false;
        let mut was_empty_last_tick = true;

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(PlayerCommand::Play(path)) => {
                    let file = match std::fs::File::open(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            let _ = event_tx
                                .send(PlayerEvent::Error(format!("couldn't open file: {e}")));
                            // Si el archivo no existe o no se puede abrir, saltamos al siguiente
                            let _ = event_tx.send(PlayerEvent::Finished);
                            continue;
                        }
                    };

                    match Decoder::new(std::io::BufReader::new(file)) {
                        Ok(source) => {
                            sink.stop();
                            sink.append(source);
                            sink.play();
                            let duration = read_duration(&path);
                            has_track = true;
                            was_empty_last_tick = false;
                            let _ = event_tx.send(PlayerEvent::Started { path, duration });
                        }
                        Err(e) => {
                            // MANEJO DE ERRORES: Si la canción está corrupta, reportamos
                            // y enviamos Finished para que el randomizador dispare la siguiente
                            let _ = event_tx
                                .send(PlayerEvent::Error(format!("Skip corrupt track: {e}")));
                            let _ = event_tx.send(PlayerEvent::Finished);
                        }
                    }
                }
                Ok(PlayerCommand::TogglePause) => {
                    if has_track {
                        if sink.is_paused() {
                            sink.play();
                            let _ = event_tx.send(PlayerEvent::Paused(false));
                        } else {
                            sink.pause();
                            let _ = event_tx.send(PlayerEvent::Paused(true));
                        }
                    }
                }
                Ok(PlayerCommand::Stop) => {
                    sink.stop();
                    has_track = false;
                    was_empty_last_tick = true;
                }
                Ok(PlayerCommand::SetVolume(v)) => {
                    sink.set_volume(v.clamp(0.0, 2.0));
                }
                Ok(PlayerCommand::SeekBy(delta)) => {
                    if has_track {
                        let pos = sink.get_pos();
                        let new_pos = if delta < 0.0 {
                            pos.saturating_sub(Duration::from_secs_f32(-delta))
                        } else {
                            pos + Duration::from_secs_f32(delta)
                        };
                        let _ = sink.try_seek(new_pos);
                    }
                }
                Ok(PlayerCommand::Shutdown) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if has_track {
                        if !sink.is_paused() {
                            let _ = event_tx.send(PlayerEvent::Position(sink.get_pos()));
                        }
                        let empty_now = sink.empty();
                        if empty_now && !was_empty_last_tick {
                            was_empty_last_tick = true;
                            has_track = false;
                            let _ = event_tx.send(PlayerEvent::Finished);
                        } else {
                            was_empty_last_tick = empty_now;
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    cmd_tx
}
