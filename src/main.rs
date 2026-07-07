use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;
use crate::backend::anthropic::AnthropicBackend;
use crate::backend::ollama::OllamaBackend;
use crate::backend::openai_compat::OpenAiCompatBackend;
use crate::backend::ChatBackend;
use crate::channel::Channel;
use crate::config::Config;
use crate::state::AppState;

mod app;
mod backend;
mod channel;
mod config;
mod models;
mod state;
mod theme;
mod tui;

fn build_backends(config: &Config) -> HashMap<String, Arc<dyn ChatBackend>> {
    let mut backends: HashMap<String, Arc<dyn ChatBackend>> = HashMap::new();

    let openai_compat_providers: &[(&str, &str)] = &[
        ("openai", "https://api.openai.com/v1"),
        ("deepseek", "https://api.deepseek.com/v1"),
        ("qwen", "https://dashscope.aliyuncs.com/compatible-mode/v1"),
    ];

    for (name, default_url) in openai_compat_providers {
        if let Some(cfg) = config.provider(name) {
            if let Some(ref api_key) = cfg.api_key {
                let base_url = if cfg.base_url.is_empty() {
                    default_url.to_string()
                } else {
                    cfg.base_url.clone()
                };
                let backend = OpenAiCompatBackend::new(
                    name.to_string(),
                    api_key.clone(),
                    base_url,
                    cfg.models.clone(),
                );
                backends.insert(name.to_string(), Arc::new(backend));
            }
        }
    }

    if let Some(cfg) = config.provider("anthropic") {
        if let Some(ref api_key) = cfg.api_key {
            let backend = AnthropicBackend::new(api_key.clone(), cfg.models.clone());
            backends.insert("anthropic".to_string(), Arc::new(backend));
        }
    }

    if let Some(cfg) = config.provider("ollama") {
        let base_url = if cfg.base_url.is_empty() {
            "http://localhost:11434".to_string()
        } else {
            cfg.base_url.clone()
        };
        let backend = OllamaBackend::new(base_url, cfg.models.clone());
        backends.insert("ollama".to_string(), Arc::new(backend));
    }

    backends
}

async fn fetch_models_for_backends(backends: &HashMap<String, Arc<dyn ChatBackend>>) {
    for (name, backend) in backends {
        match backend.fetch_models().await {
            Ok(models) if !models.is_empty() => {
                backend.set_models(models.clone());
                tracing::info!("Fetched {} models for {}", models.len(), name);
            }
            Ok(_) => {
                tracing::warn!("No models returned from {}", name);
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models for {}: {}", name, e);
            }
        }
    }
}

fn set_title(provider: &str, model: &str) {
    print!("\x1b]0;Forget - {} / {}\x07", provider, model);
    let _ = io::stdout().flush();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let mut config = Config::load()?;

    if let Some(state) = AppState::load() {
        if config.providers.contains_key(&state.provider) {
            config.default_provider = state.provider;
        }
        config.default_model = state.model;
        if !state.theme.is_empty() {
            config.theme = state.theme;
        }
    }

    let backends = build_backends(&config);

    if backends.is_empty() {
        anyhow::bail!("No providers configured. Set up at least one provider in config.toml");
    }

    fetch_models_for_backends(&backends).await;

    let theme = crate::theme::by_name(&config.theme)
        .cloned()
        .unwrap_or(crate::theme::DARK.clone());

    let channel: Option<Arc<dyn Channel>> = None;
    let mut app = App::new(config, theme, backends, channel);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(16);
    let res = run_event_loop(&mut terminal, &mut app, tick_rate);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tick_rate: Duration,
) -> anyhow::Result<()> {
    set_title(&app.current_session().provider_name, &app.current_session().model);

    let _frame_min = Duration::from_micros(16_667);
    let mut last_draw = Instant::now();
    app.needs_draw = true;

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(last_draw);
        let can_draw = !app.is_streaming || elapsed >= _frame_min;

        if app.needs_draw && can_draw {
            let theme = app.theme.clone();
            terminal.draw(|f| tui::render(f, app, &theme))?;
            app.needs_draw = false;
            last_draw = Instant::now();
        }

        if app.quit {
            return Ok(());
        }

        let poll_timeout = if app.is_streaming { Duration::from_millis(1) } else { tick_rate };

        if event::poll(poll_timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    handle_key(app, key);
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => app.scroll_up(3),
                    MouseEventKind::ScrollDown => app.scroll_down(3),
                    _ => {}
                },
                Event::Resize(_, _) => {
                    app.cache_width = 0;
                    app.needs_draw = true;
                }
                _ => {}
            }
        }

        app.poll_stream();
    }
}

fn handle_key(app: &mut App, key: crossterm::event::KeyEvent) {
    if app.picker.is_some() {
        match key.code {
            KeyCode::Esc => { app.picker_cancel(); return; }
            KeyCode::Enter => { app.picker_confirm(); return; }
            KeyCode::Up => { app.picker_select_up(); return; }
            KeyCode::Down => { app.picker_select_down(); return; }
            _ => { app.picker_cancel(); return; }
        }
    }

    if key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::SHIFT) {
        app.insert_char('\n');
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('w') => app.delete_word(),
            KeyCode::Char('a') => app.cursor_home(),
            KeyCode::Char('e') => app.cursor_end(),
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.show_help = false;
            app.needs_draw = true;
        }
        KeyCode::Enter => app.handle_enter(),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Delete => app.delete_forward(),

        KeyCode::Left => app.cursor_left(),
        KeyCode::Right => app.cursor_right(),
        KeyCode::Home => app.cursor_home(),
        KeyCode::End => {
            if app.input.is_empty() { app.scroll_to_bottom(); } else { app.cursor_end(); }
        }

        KeyCode::Up => app.scroll_up(3),
        KeyCode::Down => app.scroll_down(3),
        KeyCode::PageUp => app.scroll_up(10),
        KeyCode::PageDown => app.scroll_down(10),

        KeyCode::Tab => app.next_session(),
        KeyCode::BackTab => app.prev_session(),

        KeyCode::Char(c) => app.insert_char(c),
        _ => {}
    }
}
