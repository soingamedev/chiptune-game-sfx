use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{App, Focus},
    audio::PlaybackState,
};

const GREEN: Color = Color::Rgb(82, 255, 122);
const CYAN: Color = Color::Rgb(83, 224, 255);
const PINK: Color = Color::Rgb(255, 79, 216);
const YELLOW: Color = Color::Rgb(255, 232, 91);
const DIM: Color = Color::Rgb(95, 109, 117);

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let shell = Block::default()
        .title(Line::from(vec![
            Span::styled(" SOIN GAME DEV ", Style::new().black().bg(GREEN).bold()),
            Span::styled(" 8-BIT SFX PLAYER ", Style::new().fg(PINK).bold()),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(GREEN));
    frame.render_widget(shell, area);

    let inner = area.inner(ratatui::layout::Margin {
        horizontal: 2,
        vertical: 1,
    });

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(inner);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(vertical[0]);

    render_categories(frame, app, top[0]);
    render_tracks(frame, app, top[1]);
    render_now_playing(frame, app, vertical[1]);
    render_help(frame, app, vertical[2]);
}

fn render_categories(frame: &mut Frame, app: &App, area: Rect) {
    let items = app
        .library
        .categories
        .iter()
        .enumerate()
        .map(|(index, category)| {
            let count = category.tracks.len();
            let text = format!("{}  [{}]", category.name, count);
            if index == app.selected_category {
                ListItem::new(text).style(Style::new().fg(Color::Black).bg(GREEN).bold())
            } else {
                ListItem::new(text).style(Style::new().fg(CYAN))
            }
        })
        .collect::<Vec<_>>();

    let mut state = ListState::default();
    state.select(Some(app.selected_category));
    let title = panel_title("CATEGORIES", app.focus == Focus::Categories);
    let list = List::new(items)
        .block(panel_block(title, app.focus == Focus::Categories))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_tracks(frame: &mut Frame, app: &App, area: Rect) {
    let tracks = app.visible_tracks();
    let items = tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            if index == app.selected_track {
                ListItem::new(track.label.clone())
                    .style(Style::new().fg(Color::Black).bg(PINK).bold())
            } else {
                ListItem::new(track.label.clone()).style(Style::new().fg(GREEN))
            }
        })
        .collect::<Vec<_>>();

    let mut state = ListState::default();
    if !tracks.is_empty() {
        state.select(Some(app.selected_track.min(tracks.len() - 1)));
    }

    let title = if app.search_query.is_empty() {
        panel_title("SOUNDS", app.focus == Focus::Tracks)
    } else {
        let mode = if app.search_active {
            "SEARCHING"
        } else {
            "FILTER"
        };
        Line::from(vec![
            Span::styled(format!(" {mode}: "), Style::new().fg(YELLOW).bold()),
            Span::styled(app.search_query.as_str(), Style::new().fg(Color::White)),
            Span::styled(" ", Style::new()),
        ])
    };

    let list = List::new(items)
        .block(panel_block(title, app.focus == Focus::Tracks))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_now_playing(frame: &mut Frame, app: &App, area: Rect) {
    let state_text = match app.playback_state {
        PlaybackState::Stopped => "STOPPED",
        PlaybackState::Playing => "PLAYING",
        PlaybackState::Paused => "PAUSED",
        PlaybackState::Finished => "FINISHED",
    };
    let state_color = match app.playback_state {
        PlaybackState::Playing => GREEN,
        PlaybackState::Paused => YELLOW,
        PlaybackState::Finished => CYAN,
        PlaybackState::Stopped => DIM,
    };

    let now = app.now_playing.as_deref().unwrap_or("No sound selected");
    let rows = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(area);

    let text = vec![
        Line::from(vec![
            Span::styled("NOW PLAYING  ", Style::new().fg(DIM).bold()),
            Span::styled(now, Style::new().fg(Color::White).bold()),
        ]),
        Line::from(vec![
            Span::styled("STATUS       ", Style::new().fg(DIM).bold()),
            Span::styled(state_text, Style::new().fg(state_color).bold()),
            Span::styled("  //  ", Style::new().fg(DIM)),
            Span::styled(&app.message, Style::new().fg(CYAN)),
        ]),
        Line::from(vec![
            Span::styled("ROOT         ", Style::new().fg(DIM).bold()),
            Span::styled(app.audio_root.display().to_string(), Style::new().fg(DIM)),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(panel_block(panel_title("DECK", false), false))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, rows[0]);

    let gauge = Gauge::default()
        .block(panel_block(panel_title("VOLUME", false), false))
        .gauge_style(Style::new().fg(PINK).bg(Color::Black).bold())
        .ratio(f64::from(app.volume_percent()) / 100.0)
        .label(format!("{}%", app.volume_percent()));
    frame.render_widget(gauge, rows[1]);
}

fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let search_hint = if app.search_active {
        "typing search // Enter play // Esc leave search"
    } else {
        "↑↓ nav // ←→ panel // Enter play // Space pause // s stop // / search // +/- volume // q quit"
    };

    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled(" controls ", Style::new().black().bg(CYAN).bold()),
        Span::raw(" "),
        Span::styled(search_hint, Style::new().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(DIM)),
    );
    frame.render_widget(paragraph, area);
}

fn panel_block<'a>(title: Line<'a>, focused: bool) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(if focused { YELLOW } else { DIM }))
}

fn panel_title(title: &'static str, focused: bool) -> Line<'static> {
    let color = if focused { YELLOW } else { CYAN };
    Line::from(Span::styled(
        format!(" {title} "),
        Style::new().fg(color).bold(),
    ))
}
