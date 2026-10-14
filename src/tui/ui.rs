use crate::cli::send_command;
use crate::daemon::state::{IPCCommand, IPCResponse, TrackInfo};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph},
    Frame,
};

const COLOR_BG: Color = Color::Rgb(18, 18, 18);
const COLOR_HIGHLIGHT: Color = Color::Rgb(29, 185, 84);
const COLOR_TEXT: Color = Color::White;
const COLOR_DIM: Color = Color::Gray;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Navigation,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusPane {
    Playlists,
    Tracks,
}

pub struct App {
    pub port: u16,
    pub input_mode: InputMode,
    pub focus: FocusPane,
    pub playlists: Vec<String>,
    pub playlist_state: ListState,
    pub tracks: Vec<TrackInfo>,
    pub track_state: ListState,
    pub search_input: String,
    pub search_results: Vec<TrackInfo>,
    pub search_dirty: bool,
    pub last_search_edit: Option<std::time::Instant>,
    pub current_track: String,
    pub current_artist: String,
    pub is_playing: bool,
    pub progress_ms: u64,
    pub duration_ms: u64,
    pub status_msg: String,
    pub color_bg: Color,
    pub color_highlight: Color,
}

fn parse_hex_color(hex: &str, default: Color) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }
    default
}

impl App {
    pub async fn new(port: u16) -> Self {
        let config = crate::config::load_config();
        let color_bg = parse_hex_color(&config.ui.color_bg, COLOR_BG);
        let color_highlight = parse_hex_color(&config.ui.color_highlight, COLOR_HIGHLIGHT);

        let mut app = Self {
            port,
            input_mode: InputMode::Navigation,
            focus: FocusPane::Playlists,
            playlists: vec!["Liked Songs".to_string(), "Discover Weekly".to_string()],
            playlist_state: ListState::default(),
            tracks: Vec::new(),
            track_state: ListState::default(),
            search_input: String::new(),
            search_results: Vec::new(),
            search_dirty: false,
            last_search_edit: None,
            current_track: String::new(),
            current_artist: String::new(),
            is_playing: false,
            progress_ms: 0,
            duration_ms: 0,
            status_msg: "Daemon Active".to_string(),
            color_bg,
            color_highlight,
        };
        app.playlist_state.select(Some(0));
        app.tick().await;
        app
    }

    pub async fn tick(&mut self) {
        if let Ok(IPCResponse::State(state)) = send_command(self.port, IPCCommand::Status).await {
            self.apply_state(state);
        }
    }

    pub fn apply_state(&mut self, state: crate::daemon::state::PlaybackState) {
        self.is_playing = state.state == "playing";
        self.current_track = state.track;
        self.current_artist = state.artist;
        self.progress_ms = state.progress_ms;
        self.duration_ms = state.duration_ms;
    }

    pub async fn toggle_play_pause(&mut self) {
        let cmd = if self.is_playing {
            IPCCommand::Pause
        } else {
            IPCCommand::Play
        };
        if send_command(self.port, cmd).await.is_ok() {
            self.is_playing = !self.is_playing;
        }
        self.tick().await;
    }

    pub async fn next_track(&mut self) {
        let _ = send_command(self.port, IPCCommand::Next).await;
        self.tick().await;
    }

    pub async fn prev_track(&mut self) {
        let _ = send_command(self.port, IPCCommand::Prev).await;
        self.tick().await;
    }

    pub async fn seek(&mut self, delta_secs: i64) {
        let current_secs = (self.progress_ms / 1000) as i64;
        let new_secs = (current_secs + delta_secs).max(0) as u32;
        let _ = send_command(self.port, IPCCommand::Seek(new_secs)).await;
        self.tick().await;
    }

    pub async fn check_debounced_search(&mut self) {
        if self.search_dirty {
            if let Some(last_edit) = self.last_search_edit {
                if last_edit.elapsed() >= std::time::Duration::from_millis(250) {
                    self.search_dirty = false;
                    self.live_search().await;
                }
            }
        }
    }

    pub async fn live_search(&mut self) {
        if self.search_input.trim().is_empty() {
            self.search_results.clear();
            return;
        }

        let cmd = IPCCommand::Search {
            query: self.search_input.clone(),
            search_type: Some("track".to_string()),
            play: false,
        };

        if let Ok(IPCResponse::SearchResults(results)) = send_command(self.port, cmd).await {
            self.search_results = results;
            if !self.search_results.is_empty() {
                self.track_state.select(Some(0));
            }
        }
    }

    pub async fn play_selected(&mut self) {
        let track = if !self.search_results.is_empty() {
            self.track_state
                .selected()
                .and_then(|i| self.search_results.get(i))
        } else {
            self.track_state.selected().and_then(|i| self.tracks.get(i))
        };

        if let Some(track) = track {
            let uri = track.uri.clone();
            let _ = send_command(self.port, IPCCommand::PlayUri(uri)).await;
            self.tick().await;
        }
    }

    pub fn progress_ratio(&self) -> f64 {
        if self.duration_ms == 0 {
            0.0
        } else {
            (self.progress_ms as f64 / self.duration_ms as f64).min(1.0)
        }
    }

    pub fn format_time(ms: u64) -> String {
        let secs = ms / 1000;
        format!("{}:{:02}", secs / 60, secs % 60)
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(5),
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    render_playback(frame, app, chunks[2]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let status_color = if app.is_playing {
        app.color_highlight
    } else {
        COLOR_DIM
    };

    let search_style = if app.input_mode == InputMode::Search {
        Style::default()
            .fg(app.color_highlight)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_DIM)
    };

    let search_text = if app.input_mode == InputMode::Search {
        format!("/ {}_", app.search_input)
    } else {
        "/ Search...".to_string()
    };

    let line = Line::from(vec![
        Span::styled(
            "🟢 Spotiline  ",
            Style::default()
                .fg(app.color_highlight)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("| Mode: TUI ", Style::default().fg(COLOR_TEXT)),
        Span::styled(
            format!("| Status: {} ", app.status_msg),
            Style::default().fg(status_color),
        ),
        Span::styled(format!("| {}", search_text), search_style),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.color_highlight))
        .style(Style::default().bg(app.color_bg));

    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, area);
}

fn render_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let playlist_items: Vec<ListItem> = app
        .playlists
        .iter()
        .map(|p| {
            ListItem::new(Line::from(Span::styled(
                p.clone(),
                Style::default().fg(COLOR_TEXT),
            )))
        })
        .collect();

    let playlists = List::new(playlist_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Your Library")
                .border_style(Style::default().fg(COLOR_DIM)),
        )
        .highlight_style(
            Style::default()
                .fg(app.color_highlight)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(playlists, chunks[0], &mut app.playlist_state);

    let display_tracks: Vec<ListItem> = if !app.search_results.is_empty() {
        app.search_results
            .iter()
            .enumerate()
            .map(|(i, t)| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{}. ", i + 1), Style::default().fg(COLOR_DIM)),
                    Span::styled(
                        format!("{} - {}", t.name, t.artist),
                        Style::default().fg(COLOR_TEXT),
                    ),
                ]))
            })
            .collect()
    } else if !app.tracks.is_empty() {
        app.tracks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{}. ", i + 1), Style::default().fg(COLOR_DIM)),
                    Span::styled(
                        format!("{} - {}", t.name, t.artist),
                        Style::default().fg(COLOR_TEXT),
                    ),
                ]))
            })
            .collect()
    } else {
        vec![ListItem::new(Line::from(Span::styled(
            "Press / to search for songs, artists, or albums",
            Style::default().fg(COLOR_DIM),
        )))]
    };

    let title = if !app.search_results.is_empty() {
        "Search Results"
    } else {
        "Tracks"
    };
    let tracks = List::new(display_tracks)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(COLOR_DIM)),
        )
        .highlight_style(
            Style::default()
                .fg(app.color_highlight)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(tracks, chunks[1], &mut app.track_state);
}

fn render_playback(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.color_highlight))
        .style(Style::default().bg(app.color_bg));
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let play_icon = if app.is_playing { "⏸ " } else { "▶ " };
    let track_info = if app.current_track.is_empty() {
        "No track playing  •  Press / to search, Space to play/pause".to_string()
    } else {
        format!(
            "{}{} - {}",
            play_icon, app.current_track, app.current_artist
        )
    };

    let progress_text = format!(
        "{} / {}",
        App::format_time(app.progress_ms),
        App::format_time(app.duration_ms)
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner_area);

    let info = Paragraph::new(Line::from(Span::styled(
        track_info,
        Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(info, chunks[0]);

    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(app.color_highlight).bg(app.color_bg))
        .ratio(app.progress_ratio())
        .label(progress_text);
    frame.render_widget(gauge, chunks[1]);

    let help = Paragraph::new(Line::from(vec![Span::styled(
        "Space: Play/Pause | ←/→: Seek 10s | Enter: Play | /: Search | Tab: Focus | q: Quit",
        Style::default().fg(COLOR_DIM),
    )]));
    frame.render_widget(help, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color_valid() {
        assert_eq!(
            parse_hex_color("#1DB954", Color::White),
            Color::Rgb(29, 185, 84)
        );
        assert_eq!(
            parse_hex_color("1DB954", Color::White),
            Color::Rgb(29, 185, 84)
        );
        assert_eq!(
            parse_hex_color("#000000", Color::White),
            Color::Rgb(0, 0, 0)
        );
        assert_eq!(
            parse_hex_color("#FFFFFF", Color::Black),
            Color::Rgb(255, 255, 255)
        );
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert_eq!(parse_hex_color("invalid", Color::White), Color::White);
        assert_eq!(parse_hex_color("", Color::Black), Color::Black);
        assert_eq!(parse_hex_color("#12345", Color::Red), Color::Red);
    }
}
