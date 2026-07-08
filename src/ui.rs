use std::time::Duration;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, PlayState, View};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let root = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tab bar
            Constraint::Min(3),    // main content
            Constraint::Length(3), // now-playing mini bar
            Constraint::Length(1), // status/help line
        ])
        .split(root);

    draw_tab_bar(frame, chunks[0], app);

    match app.view {
        View::Browser => draw_browser(frame, chunks[1], app),
        View::NowPlaying => draw_now_playing(frame, chunks[1], app),
    }

    draw_mini_player(frame, chunks[2], app);
    draw_status_line(frame, chunks[3], app);
}

fn draw_tab_bar(frame: &mut Frame, area: Rect, app: &App) {
    let make = |label: &str, active: bool| {
        let style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        Span::styled(format!(" {label} "), style)
    };

    let line = Line::from(vec![
        make("[1] Browser", app.view == View::Browser),
        Span::raw(" "),
        make("[2] Now Playing", app.view == View::NowPlaying),
        Span::raw("   FUSKYOM — Terminal Music Player"),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_browser(frame: &mut Frame, area: Rect, app: &mut App) {
    let area = if app.search_active {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(area);
        draw_search_box(frame, split[0], app);
        split[1]
    } else {
        area
    };

    let visible = app.visible_entries();
    let items: Vec<ListItem> = visible
        .iter()
        .map(|e| {
            let label = if e.is_dir {
                format!("📁 {}/", e.name)
            } else {
                format!("🎵 {}", e.name)
            };
            let style = if e.is_dir {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Span::styled(label, style))
        })
        .collect();

    let title = if app.search_query.is_empty() {
        format!(" {} ", app.cwd.display())
    } else {
        format!(" {} ({} results) ", app.cwd.display(), items.len())
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_search_box(frame: &mut Frame, area: Rect, app: &App) {
    let text = format!("/{}", app.search_query);
    let box_widget = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Search (Esc or / to exit) ")
            .style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(box_widget, area);
}

fn draw_now_playing(frame: &mut Frame, area: Rect, app: &mut App) {
    // Terminal character cells are roughly twice as tall as they are wide, so
    // a square album cover needs about `height * 2` columns to fill its box
    // without chafa having to letterbox it (which is what left that big dead
    // gap before). We derive the art column's width from the pane's height
    // instead of a fixed percentage, so the reserved space actually matches
    // what a square cover needs -- clamped so it never eats more than 70% of
    // the width on a very short/wide terminal, nor collapses below something
    // usable on a very tall/narrow one.
    let width = area.width as u32;
    let height = area.height as u32;
    let max_allowed = (width * 7 / 10).max(15);
    let art_width = (height * 2)
        .clamp(15, max_allowed)
        .min(width.saturating_sub(15))
        .max(10) as u16;

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(art_width), Constraint::Min(20)])
        .split(area);

    // Left: album art via chafa.
    let art_block = Block::default().borders(Borders::ALL).title(" Album Art ");
    let inner = art_block.inner(cols[0]);
    frame.render_widget(art_block, cols[0]);

    if let Some(track) = app.current_track.clone() {
        // Leave a little margin so chafa's block glyphs don't touch the border.
        let w = inner.width.saturating_sub(1);
        let h = inner.height.saturating_sub(1);
        if let Some(art) = app.art.render(&track, w, h) {
            frame.render_widget(Paragraph::new(art), inner);
        } else if app.art.chafa_missing {
            frame.render_widget(
                Paragraph::new("chafa not installed.\nsudo apt install chafa"),
                inner,
            );
        } else {
            frame.render_widget(Paragraph::new("(no embedded cover art)"), inner);
        }
    } else {
        frame.render_widget(Paragraph::new("Nothing playing"), inner);
    }

    // Right: track info + queue.
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(3)])
        .split(cols[1]);

    let info_lines = track_info_lines(app);
    frame.render_widget(
        Paragraph::new(info_lines).block(Block::default().borders(Borders::ALL).title(" Track ")),
        right[0],
    );

    let queue_items: Vec<ListItem> = app
        .queue
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let is_current = app.queue_pos == Some(i);
            let label = if is_current {
                format!("▶ {name}")
            } else {
                format!("  {name}")
            };
            let style = if is_current {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(label, style))
        })
        .collect();

    frame.render_widget(
        List::new(queue_items).block(Block::default().borders(Borders::ALL).title(" Cola ")),
        right[1],
    );
}

fn track_info_lines(app: &App) -> Vec<Line<'static>> {
    let Some(track) = &app.current_track else {
        return vec![Line::from("No track playing")];
    };
    let name = track
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let state = match app.play_state {
        PlayState::Playing => "▶ playing",
        PlayState::Paused => "⏸ paused",
        PlayState::Stopped => "⏹ stopped",
    };
    vec![
        Line::from(Span::styled(
            name,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "{state}    vol {:.0}%    repeat {}",
            app.volume * 100.0,
            if app.repeat { "on" } else { "off" }
        )),
    ]
}

fn draw_mini_player(frame: &mut Frame, area: Rect, app: &App) {
    let pos = fmt_duration(app.position);
    let dur = app
        .current_duration
        .map(fmt_duration)
        .unwrap_or_else(|| "--:--".to_string());

    let name = app
        .current_track
        .as_ref()
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "— no track playing —".to_string());

    let ratio = match (app.current_duration, app.current_track.is_some()) {
        (Some(total), true) if total.as_secs_f64() > 0.0 => {
            (app.position.as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };

    let state_symbol = match app.play_state {
        PlayState::Playing => "▶",
        PlayState::Paused => "⏸",
        PlayState::Stopped => "⏹",
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {state_symbol} {name} ")),
        )
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .ratio(ratio)
        .label(format!("{pos} / {dur}"));

    frame.render_widget(gauge, area);
}

fn draw_status_line(frame: &mut Frame, area: Rect, app: &App) {
    let help = "(<-/->) move | (/) manual search | (Enter) play | (space) pause | (n/p) next/prev | (s) stop | (+/-) vol | (r) repeat | (1/2) or (Tab) view | (q) exit";
    let line = if app.status.is_empty() {
        help.to_string()
    } else {
        format!("{}   |   {help}", app.status)
    };
    frame.render_widget(
        Paragraph::new(line).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

fn fmt_duration(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}
