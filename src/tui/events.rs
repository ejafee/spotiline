use super::ui::{App, FocusPane, InputMode};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

pub async fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<bool> {
    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }

    match app.input_mode {
        InputMode::Navigation => handle_navigation_key(app, key).await,
        InputMode::Search => handle_search_key(app, key).await,
    }
}

async fn handle_navigation_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char(' ') => {
            app.toggle_play_pause().await;
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
        }
        KeyCode::Enter => {
            app.play_selected().await;
        }
        KeyCode::Left => {
            app.seek(-10).await;
        }
        KeyCode::Right => {
            app.seek(10).await;
        }
        KeyCode::Up => {
            move_selection_up(app);
        }
        KeyCode::Down => {
            move_selection_down(app);
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                FocusPane::Playlists => FocusPane::Tracks,
                FocusPane::Tracks => FocusPane::Playlists,
            };
        }
        KeyCode::Char('n') => {
            app.next_track().await;
        }
        KeyCode::Char('p') => {
            app.prev_track().await;
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_search_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Navigation;
            app.search_input.clear();
            app.search_results.clear();
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Navigation;
            if !app.search_results.is_empty() {
                app.play_selected().await;
            }
        }
        KeyCode::Backspace => {
            app.search_input.pop();
            app.live_search().await;
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
            app.live_search().await;
        }
        KeyCode::Up => {
            move_selection_up(app);
        }
        KeyCode::Down => {
            move_selection_down(app);
        }
        _ => {}
    }
    Ok(false)
}

fn move_selection_up(app: &mut App) {
    match app.focus {
        FocusPane::Playlists => {
            let i = app.playlist_state.selected().unwrap_or(0);
            if i > 0 {
                app.playlist_state.select(Some(i - 1));
            }
        }
        FocusPane::Tracks => {
            let i = app.track_state.selected().unwrap_or(0);
            if i > 0 {
                app.track_state.select(Some(i - 1));
            }
        }
    }
}

fn move_selection_down(app: &mut App) {
    match app.focus {
        FocusPane::Playlists => {
            let i = app.playlist_state.selected().unwrap_or(0);
            let len = app.playlists.len();
            if i + 1 < len {
                app.playlist_state.select(Some(i + 1));
            }
        }
        FocusPane::Tracks => {
            let i = app.track_state.selected().unwrap_or(0);
            let len = if !app.search_results.is_empty() {
                app.search_results.len()
            } else {
                app.tracks.len()
            };
            if i + 1 < len {
                app.track_state.select(Some(i + 1));
            }
        }
    }
}
