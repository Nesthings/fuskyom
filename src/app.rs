use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::art::ArtRenderer;
use crate::library::{self, Entry};
use crate::player::{PlayerCommand, PlayerEvent};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum View {
    Browser,
    NowPlaying,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PlayState {
    Stopped,
    Playing,
    Paused,
}

pub struct App {
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: usize,

    pub view: View,
    pub should_quit: bool,

    pub search_active: bool,
    pub search_query: String,

    pub queue: Vec<PathBuf>,
    pub queue_pos: Option<usize>,

    pub play_state: PlayState,
    pub current_track: Option<PathBuf>,
    pub current_duration: Option<Duration>,
    pub position: Duration,
    pub volume: f32,
    pub repeat: bool,

    pub status: String,
    pub art: ArtRenderer,

    cmd_tx: Sender<PlayerCommand>,
}

impl App {
    pub fn new(start_dir: PathBuf, cmd_tx: Sender<PlayerCommand>) -> Self {
        let entries = library::read_dir_sorted(&start_dir).unwrap_or_default();
        Self {
            cwd: start_dir,
            entries,
            selected: 0,
            view: View::Browser,
            should_quit: false,
            search_active: false,
            search_query: String::new(),
            queue: Vec::new(),
            queue_pos: None,
            play_state: PlayState::Stopped,
            current_track: None,
            current_duration: None,
            position: Duration::ZERO,
            volume: 1.0,
            repeat: false,
            status: "Welcome to FUSKYOM — h keys help below".to_string(),
            art: ArtRenderer::new(),
            cmd_tx,
        }
    }

    fn refresh_entries(&mut self) {
        self.entries = library::read_dir_sorted(&self.cwd).unwrap_or_default();
        self.selected = 0;
        self.search_active = false;
        self.search_query.clear();
    }

    /// Entries currently shown in the browser: everything, unless a search
    /// filter is active, in which case only entries whose name contains the
    /// query (case-insensitive) are kept.
    pub fn visible_entries(&self) -> Vec<&Entry> {
        if self.search_query.is_empty() {
            self.entries.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.entries
                .iter()
                .filter(|e| e.name.to_lowercase().contains(&q))
                .collect()
        }
    }

    fn clamp_selection(&mut self) {
        let len = self.visible_entries().len();
        self.selected = if len == 0 {
            0
        } else {
            self.selected.min(len - 1)
        };
    }

    pub fn selected_entry(&self) -> Option<Entry> {
        self.visible_entries()
            .get(self.selected)
            .map(|e| (*e).clone())
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = self.visible_entries().len();
        if len == 0 {
            return;
        }
        let len = len as i32;
        let mut idx = self.selected as i32 + delta;
        idx = idx.clamp(0, len - 1);
        self.selected = idx as usize;
    }

    /// Toggles the "/" search box: opens it fresh if it was closed, or
    /// closes it (clearing the query and restoring the full list) if it was
    /// already open -- pressing "/" a second time gets you out cleanly so it
    /// stops eating keystrokes meant for playback commands.
    pub fn toggle_search(&mut self) {
        if self.search_active {
            self.close_search();
        } else {
            self.search_active = true;
            self.search_query.clear();
            self.selected = 0;
        }
    }

    fn close_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.selected = 0;
    }

    /// Key handling while the search box is focused: typed characters build
    /// the query instead of triggering playback commands, but Up/Down still
    /// navigate the filtered results so you can arrow down straight into a
    /// match without leaving search mode.
    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('/') => self.close_search(),
            KeyCode::Enter => {
                self.search_active = false;
                self.enter_or_play();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.clamp_selection();
            }
            KeyCode::Down => self.move_selection(1),
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.clamp_selection();
            }
            _ => {}
        }
    }

    pub fn enter_or_play(&mut self) {
        let Some(entry) = self.selected_entry() else {
            return;
        };
        if entry.is_dir {
            self.cwd = entry.path;
            self.refresh_entries();
        } else {
            self.play_from_dir_at(&entry.path);
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            let parent = parent.to_path_buf();
            self.cwd = parent;
            self.refresh_entries();
        }
    }

    /// Builds the queue from every audio file in the same directory as
    /// `track`, starting at `track` itself, then starts playback -- this is
    /// the same "play this, keep going through the folder" behavior cmus has
    /// when you hit Enter in the library view.
    fn play_from_dir_at(&mut self, track: &PathBuf) {
        let dir = track.parent().unwrap_or(&self.cwd);
        let files = library::audio_files_in(dir).unwrap_or_default();
        let start = files.iter().position(|p| p == track).unwrap_or(0);
        self.queue = files;
        self.queue_pos = Some(start);
        self.play_current_queue_item();
    }

    fn play_current_queue_item(&mut self) {
        if let Some(pos) = self.queue_pos {
            if let Some(path) = self.queue.get(pos).cloned() {
                let _ = self.cmd_tx.send(PlayerCommand::Play(path));
                self.view = View::NowPlaying;
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.current_track.is_some() {
            let _ = self.cmd_tx.send(PlayerCommand::TogglePause);
        }
    }

    pub fn stop(&mut self) {
        let _ = self.cmd_tx.send(PlayerCommand::Stop);
        self.play_state = PlayState::Stopped;
        self.current_track = None;
        self.position = Duration::ZERO;
    }

    pub fn next_track(&mut self) {
        if let Some(pos) = self.queue_pos {
            if pos + 1 < self.queue.len() {
                self.queue_pos = Some(pos + 1);
                self.play_current_queue_item();
            } else if self.repeat && !self.queue.is_empty() {
                self.queue_pos = Some(0);
                self.play_current_queue_item();
            } else {
                self.stop();
                self.status = "Queue end".to_string();
            }
        }
    }

    pub fn prev_track(&mut self) {
        if let Some(pos) = self.queue_pos {
            if pos > 0 {
                self.queue_pos = Some(pos - 1);
                self.play_current_queue_item();
            }
        }
    }

    pub fn adjust_volume(&mut self, delta: f32) {
        self.volume = (self.volume + delta).clamp(0.0, 2.0);
        let _ = self.cmd_tx.send(PlayerCommand::SetVolume(self.volume));
        self.status = format!("Volume: {:.0}%", self.volume * 100.0);
    }

    pub fn seek_by(&mut self, secs: f32) {
        if self.current_track.is_some() {
            let _ = self.cmd_tx.send(PlayerCommand::SeekBy(secs));
        }
    }

    pub fn on_player_event(&mut self, event: PlayerEvent) {
        match event {
            PlayerEvent::Started { path, duration } => {
                self.current_track = Some(path);
                self.current_duration = duration;
                self.position = Duration::ZERO;
                self.play_state = PlayState::Playing;
            }
            PlayerEvent::Position(pos) => {
                self.position = pos;
            }
            PlayerEvent::Paused(paused) => {
                self.play_state = if paused {
                    PlayState::Paused
                } else {
                    PlayState::Playing
                };
            }
            PlayerEvent::Finished => {
                self.next_track();
            }
            PlayerEvent::Error(msg) => {
                self.status = format!("Error: {msg}");
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.search_active {
            self.handle_search_key(key);
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            KeyCode::Char('/') => self.toggle_search(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::PageUp => self.move_selection(-10),
            KeyCode::Enter => self.enter_or_play(),
            KeyCode::Backspace | KeyCode::Char('h') | KeyCode::Left => {
                if self.view == View::Browser {
                    self.go_up();
                } else {
                    self.prev_track();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.view == View::NowPlaying {
                    self.next_track();
                } else {
                    self.enter_or_play();
                }
            }
            KeyCode::Char(' ') => self.toggle_pause(),
            KeyCode::Char('s') => self.stop(),
            KeyCode::Char('n') => self.next_track(),
            KeyCode::Char('p') => self.prev_track(),
            KeyCode::Char('+') | KeyCode::Char('=') => self.adjust_volume(0.05),
            KeyCode::Char('-') => self.adjust_volume(-0.05),
            KeyCode::Char('.') => self.seek_by(5.0),
            KeyCode::Char(',') => self.seek_by(-5.0),
            KeyCode::Char('r') => {
                self.repeat = !self.repeat;
                self.status = format!("Repeat: {}", if self.repeat { "on" } else { "off" });
            }
            KeyCode::Char('1') => self.view = View::Browser,
            KeyCode::Char('2') => self.view = View::NowPlaying,
            KeyCode::Tab => {
                self.view = match self.view {
                    View::Browser => View::NowPlaying,
                    View::NowPlaying => View::Browser,
                };
            }
            _ => {}
        }
    }
}
