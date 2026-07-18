mod app;
mod audio;
mod library;
mod ui;

use std::{env, path::PathBuf, time::Duration};

use anyhow::{bail, Context, Result};
use app::{App, Focus};
use audio::AudioPlayer;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use library::AudioLibrary;
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    let audio_root = resolve_audio_root()?;
    let library = AudioLibrary::scan(&audio_root)
        .with_context(|| format!("failed to scan audio root: {}", audio_root.display()))?;

    if library.is_empty() {
        bail!("no .ogg files found in {}", audio_root.display());
    }

    let audio = AudioPlayer::new().context("failed to open the default audio output device")?;
    let mut app = App::new(library, audio_root, audio);

    run_terminal(&mut app)
}

fn run_terminal(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = run_app_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(80))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key)?,
                _ => {}
            }
        }

        app.refresh_audio_status();
    }

    app.audio.stop();
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.search_active {
        match key.code {
            KeyCode::Esc => app.leave_search(),
            KeyCode::Enter => app.play_selected()?,
            KeyCode::Backspace => app.pop_search_char(),
            KeyCode::Up => app.previous_track(),
            KeyCode::Down => app.next_track(),
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => app.focus = Focus::Tracks,
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.push_search_char(c)
            }
            _ => {}
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::Left => app.focus = Focus::Categories,
        KeyCode::Right => app.focus = Focus::Tracks,
        KeyCode::Up => app.previous_item(),
        KeyCode::Down => app.next_item(),
        KeyCode::Enter => app.play_selected()?,
        KeyCode::Char(' ') => app.toggle_pause_or_play()?,
        KeyCode::Char('s') | KeyCode::Char('S') => app.stop(),
        KeyCode::Char('+') | KeyCode::Char('=') => app.increase_volume(),
        KeyCode::Char('-') | KeyCode::Char('_') => app.decrease_volume(),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true
        }
        _ => {}
    }

    Ok(())
}

fn resolve_audio_root() -> Result<PathBuf> {
    let mut args = env::args().skip(1);

    if let Some(arg) = args.next() {
        let audio_root = match arg.as_str() {
            "--audio-root" => PathBuf::from(args.next().context("--audio-root requires a path")?),
            "-h" | "--help" => {
                println!("Usage: soin-chiptune-player [--audio-root <path>]");
                println!(
                    "Default lookup: ../assets/audio/sfx from the current directory or executable ancestors."
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        };

        if let Some(extra) = args.next() {
            bail!("unexpected extra argument: {extra}");
        }

        return Ok(audio_root);
    }

    let current_dir = env::current_dir()?;
    let mut candidates = vec![
        current_dir.join("../assets/audio/sfx"),
        current_dir.join("assets/audio/sfx"),
    ];

    if let Ok(exe) = env::current_exe() {
        for ancestor in exe.ancestors() {
            candidates.push(ancestor.join("../assets/audio/sfx"));
            candidates.push(ancestor.join("assets/audio/sfx"));
        }
    }

    candidates
        .into_iter()
        .find(|path| path.is_dir())
        .context("could not find assets/audio/sfx; pass --audio-root <path>")
}
