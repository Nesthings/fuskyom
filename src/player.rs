use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

pub enum PlayerCommand {
    Play(PathBuf),
    TogglePause,
    Stop,
    SetVolume(f32),
    /// Relative seek in seconds, can be negative.
    SeekBy(f32),
    Shutdown,
}

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

/// Wraps a rodio Source and forwards chunks of samples to the spectrum analyzer.
struct SamplerSource<I> {
    inner: I,
    buf: Vec<f32>,
    chunk: usize,
    tx: Sender<Vec<f32>>,
}

impl<I: Source<Item = i16>> SamplerSource<I> {
    fn new(inner: I, tx: Sender<Vec<f32>>, chunk: usize) -> Self {
        Self { inner, buf: Vec::with_capacity(chunk), chunk, tx }
    }
}

impl<I: Source<Item = i16>> Iterator for SamplerSource<I> {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        let s = self.inner.next()?;
        self.buf.push(s as f32 / 32768.0);
        if self.buf.len() >= self.chunk {
            let chunk = std::mem::replace(&mut self.buf, Vec::with_capacity(self.chunk));
            let _ = self.tx.send(chunk);
        }
        Some(s)
    }
}

impl<I: Source<Item = i16>> Source for SamplerSource<I> {
    fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
    fn channels(&self) -> u16 { self.inner.channels() }
    fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
    fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
}

fn read_duration(path: &Path) -> Option<Duration> {
    use lofty::file::AudioFile;
    let tagged = lofty::probe::Probe::open(path).ok()?.read().ok()?;
    Some(tagged.properties().duration())
}

pub fn spawn(event_tx: Sender<PlayerEvent>, sample_tx: Sender<Vec<f32>>) -> Sender<PlayerCommand> {
    let (cmd_tx, cmd_rx): (Sender<PlayerCommand>, Receiver<PlayerCommand>) =
        std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let (_stream, stream_handle): (OutputStream, OutputStreamHandle) =
            match OutputStream::try_default() {
                Ok(s) => s,
                Err(e) => {
                    let _ = event_tx.send(PlayerEvent::Error(format!("cannot open audio device: {e}")));
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
                            let _ = event_tx.send(PlayerEvent::Error(format!("couldn't open file: {e}")));
                            continue;
                        }
                    };
                    match Decoder::new(std::io::BufReader::new(file)) {
                        Ok(source) => {
                            sink.stop();
                            sink.append(SamplerSource::new(source, sample_tx.clone(), 1024));
                            sink.play();
                            let duration = read_duration(&path);
                            has_track = true;
                            was_empty_last_tick = false;
                            let _ = event_tx.send(PlayerEvent::Started { path, duration });
                        }
                        Err(e) => {
                            let _ = event_tx.send(PlayerEvent::Error(format!("cannot decode: {e}")));
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
