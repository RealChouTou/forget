use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::backend::ChatBackend;
use crate::channel::Channel;
use crate::config::Config;
use crate::models::{Message, Role, SessionMessage};
use crate::state::AppState;
use crate::theme::Theme;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use ratatui::text::Line as TuiLine;

#[derive(Debug, Clone, PartialEq)]
pub enum PickerAction {
    SetModel,
    SetProvider,
    SetTheme,
    SwitchSession,
}

#[derive(Debug, Clone)]
pub struct Picker {
    pub title: String,
    pub items: Vec<String>,
    pub selected: usize,
    pub action: PickerAction,
}

pub struct Session {
    pub id: Uuid,
    pub provider_name: String,
    pub model: String,
    pub messages: Vec<Message>,
}

impl Session {
    pub fn new(provider_name: String, model: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            provider_name,
            model,
            messages: Vec::new(),
        }
    }
}

fn resolve_model(backends: &HashMap<String, Arc<dyn ChatBackend>>, provider: &str, requested: &str) -> String {
    backends
        .get(provider)
        .and_then(|b| {
            let models = b.models();
            models.iter().find(|m| m.as_str() == requested).cloned()
                .or_else(|| models.first().cloned())
        })
        .unwrap_or_default()
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
    pub backends: HashMap<String, Arc<dyn ChatBackend>>,
    pub sessions: Vec<Session>,
    pub current_session: usize,
    pub input: String,
    pub cursor_byte: usize,
    pub input_scroll: usize,
    pub scroll_offset: usize,
    pub at_bottom: bool,
    pub is_streaming: bool,
    pub show_help: bool,
    pub picker: Option<Picker>,
    pub quit: bool,
    pub needs_draw: bool,
    pub channel: Option<Arc<dyn Channel>>,
    pub rendered_cache: Vec<TuiLine<'static>>,
    pub cache_msg_count: usize,
    pub cache_last_ai_len: usize,
    pub cache_width: u16,
    stream_rx: Option<mpsc::UnboundedReceiver<String>>,
    pending_user_msg: Option<Message>,
}

impl App {
    pub fn new(
        config: Config,
        theme: Theme,
        backends: HashMap<String, Arc<dyn ChatBackend>>,
        channel: Option<Arc<dyn Channel>>,
    ) -> Self {
        let model = resolve_model(&backends, &config.default_provider, &config.default_model);
        let default_session = Session::new(config.default_provider.clone(), model);

        Self {
            config,
            theme,
            backends,
            sessions: vec![default_session],
            current_session: 0,
            input: String::new(),
            cursor_byte: 0,
            input_scroll: 0,
            scroll_offset: 0,
            at_bottom: true,
            is_streaming: false,
            show_help: false,
            picker: None,
            quit: false,
            needs_draw: true,
            channel,
            rendered_cache: Vec::new(),
            cache_msg_count: 0,
            cache_last_ai_len: 0,
            cache_width: 0,
            stream_rx: None,
            pending_user_msg: None,
        }
    }

    pub fn current_session(&self) -> &Session {
        &self.sessions[self.current_session]
    }

    pub fn current_session_mut(&mut self) -> &mut Session {
        &mut self.sessions[self.current_session]
    }

    pub fn current_backend(&self) -> Option<&Arc<dyn ChatBackend>> {
        let name = &self.current_session().provider_name;
        self.backends.get(name)
    }

    fn add_system_message(&mut self, text: String) {
        self.current_session_mut().messages.push(Message::new(Role::System, text));
        self.needs_draw = true;
    }

    pub fn handle_enter(&mut self) {
        if self.is_streaming {
            return;
        }

        let trimmed = self.input.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        if trimmed.starts_with('/') {
            self.input.clear();
            self.cursor_byte = 0;
            self.needs_draw = true;
            self.execute_command(&trimmed);
        } else {
            self.send_message();
            self.needs_draw = true;
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd[1..].splitn(2, ' ').collect();
        let raw_name = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();

        let name = match raw_name.as_str() {
            "m" | "model" | "models" => "model",
            "p" | "provider" | "providers" => "provider",
            "t" | "theme" | "themes" => "theme",
            "s" | "session" | "sessions" => "session",
            "q" | "quit" | "exit" => "quit",
            "n" | "new" | "clear" => "new",
            "h" | "help" | "?" => "help",
            other => other,
        };

        match name {
            "quit" => {
                self.quit = true;
            }
            "new" => {
                self.new_session();
                self.add_system_message("new session created".into());
            }
            "model" => {
                if arg.is_empty() {
                    self.open_model_picker();
                } else {
                    self.set_model(&arg);
                }
            }
            "provider" => {
                if arg.is_empty() {
                    self.open_provider_picker();
                } else {
                    self.set_provider(&arg);
                }
            }
            "theme" => {
                if arg.is_empty() {
                    self.open_theme_picker();
                } else {
                    self.set_theme(&arg);
                }
            }
            "session" => {
                if let Ok(n) = arg.parse::<usize>() {
                    self.switch_session(n);
                } else {
                    self.open_session_picker();
                }
            }
            "help" => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.add_system_message("showing help — press Esc or /h to close".into());
                }
            }
            _ => {
                self.add_system_message(format!("unknown command: /{} — type /help for commands", raw_name));
            }
        }
        self.scroll_offset = 0;
    }

    fn set_model(&mut self, name: &str) {
        if let Some(backend) = self.current_backend() {
            let models = backend.models();
            if models.iter().any(|m| m == name) {
                self.current_session_mut().model = name.to_string();
                self.add_system_message(format!("model set to {}", name));
                self.save_state();
            } else {
                self.add_system_message(format!("model '{}' not found. try /models", name));
            }
        }
    }

    fn set_provider(&mut self, name: &str) {
        let name_lower = name.to_lowercase();
        if self.backends.contains_key(&name_lower) {
            let model = resolve_model(&self.backends, &name_lower, &self.config.default_model);
            let msg = {
                let session = self.current_session_mut();
                session.provider_name = name_lower;
                session.model = model;
                format!("provider set to {}", session.provider_name)
            };
            self.add_system_message(msg);
            self.save_state();
        } else {
            self.add_system_message(format!("provider '{}' not found. try /providers", name));
        }
    }

    fn set_theme(&mut self, name: &str) {
        if let Some(t) = crate::theme::by_name(name) {
            self.theme = t.clone();
            self.add_system_message(format!("theme set to {}", t.name));
            self.save_state();
        } else {
            self.add_system_message(format!("theme '{}' not found. try /themes", name));
        }
    }

    fn switch_session(&mut self, n: usize) {
        if n < 1 || n > self.sessions.len() {
            self.add_system_message(format!("session number 1-{}", self.sessions.len()));
            return;
        }
        self.current_session = n - 1;
        self.add_system_message(format!("switched to session {}", n));
        self.at_bottom = true;
        self.scroll_offset = 0;
    }

    fn open_model_picker(&mut self) {
        if let Some(backend) = self.current_backend() {
            let models = backend.models();
            if models.is_empty() {
                self.add_system_message("no models available".into());
                return;
            }
            self.picker = Some(Picker {
                title: format!("Models — {}", self.current_session().provider_name),
                items: models,
                selected: 0,
                action: PickerAction::SetModel,
            });
            self.needs_draw = true;
        }
    }

    fn open_provider_picker(&mut self) {
        let mut names: Vec<String> = self.backends.keys().cloned().collect();
        names.sort();
        if names.is_empty() {
            return;
        }
        self.picker = Some(Picker {
            title: "Providers".into(),
            items: names,
            selected: 0,
            action: PickerAction::SetProvider,
        });
        self.needs_draw = true;
    }

    fn open_theme_picker(&mut self) {
        let themes: Vec<String> = crate::theme::ALL.iter().map(|t| t.name.to_string()).collect();
        self.picker = Some(Picker {
            title: "Themes".into(),
            items: themes,
            selected: 0,
            action: PickerAction::SetTheme,
        });
        self.needs_draw = true;
    }

    fn open_session_picker(&mut self) {
        let items: Vec<String> = self.sessions.iter().enumerate()
            .map(|(i, s)| format!("Session {} — {} / {}", i + 1, s.provider_name, s.model))
            .collect();
        self.picker = Some(Picker {
            title: "Sessions".into(),
            items,
            selected: self.current_session,
            action: PickerAction::SwitchSession,
        });
        self.needs_draw = true;
    }

    pub fn picker_select_up(&mut self) {
        if let Some(ref mut p) = self.picker {
            if p.selected > 0 {
                p.selected -= 1;
                self.needs_draw = true;
            }
        }
    }

    pub fn picker_select_down(&mut self) {
        if let Some(ref mut p) = self.picker {
            if p.selected + 1 < p.items.len() {
                p.selected += 1;
                self.needs_draw = true;
            }
        }
    }

    pub fn picker_confirm(&mut self) {
        if let Some(picker) = self.picker.take() {
            let value = picker.items[picker.selected].clone();
            match picker.action {
                PickerAction::SetModel => self.set_model(&value),
                PickerAction::SetProvider => self.set_provider(&value),
                PickerAction::SetTheme => self.set_theme(&value),
                PickerAction::SwitchSession => {
                    if let Some(pos) = picker.items.iter().position(|s| s == &value) {
                        self.switch_session(pos + 1);
                    }
                }
            }
        }
    }

    pub fn picker_cancel(&mut self) {
        self.picker = None;
        self.needs_draw = true;
    }

    pub fn new_session(&mut self) {
        let provider = self.config.default_provider.clone();
        let model = resolve_model(&self.backends, &provider, &self.config.default_model);
        self.sessions.push(Session::new(provider, model));
        self.current_session = self.sessions.len() - 1;
        self.input.clear();
        self.cursor_byte = 0;
        self.scroll_offset = 0;
        self.at_bottom = true;
        self.needs_draw = true;
    }

    pub fn next_session(&mut self) {
        if self.sessions.len() > 1 {
            self.current_session = (self.current_session + 1) % self.sessions.len();
            self.input.clear();
            self.cursor_byte = 0;
            self.scroll_offset = 0;
            self.needs_draw = true;
        }
    }

    pub fn prev_session(&mut self) {
        if self.sessions.len() > 1 {
            self.current_session = (self.current_session + self.sessions.len() - 1)
                % self.sessions.len();
            self.input.clear();
            self.cursor_byte = 0;
            self.scroll_offset = 0;
            self.needs_draw = true;
        }
    }

    fn save_state(&self) {
        let state = AppState {
            provider: self.current_session().provider_name.clone(),
            model: self.current_session().model.clone(),
            theme: self.theme.name.to_string(),
        };
        state.save();
    }

    pub fn send_message(&mut self) {
        if self.input.trim().is_empty() || self.is_streaming {
            return;
        }

        let user_text = std::mem::take(&mut self.input);
        self.cursor_byte = 0;
        let user_msg = Message::new(Role::User, user_text.clone());
        self.pending_user_msg = Some(user_msg.clone());
        self.current_session_mut().messages.push(user_msg);

        let backend = match self.current_backend() {
            Some(b) => Arc::clone(b),
            None => return,
        };

        let model = self.current_session().model.clone();
        let messages = self.current_session().messages.clone();

        let (tx, rx) = mpsc::unbounded_channel();
        self.stream_rx = Some(rx);
        self.is_streaming = true;

        let ai_msg = Message::new(Role::Assistant, String::new());
        self.current_session_mut().messages.push(ai_msg);
        self.at_bottom = true;
        self.needs_draw = true;

        tokio::spawn(async move {
            if let Err(e) = backend.chat_stream(&model, &messages, tx).await {
                tracing::error!("Chat stream error: {}", e);
            }
        });
    }

    pub fn poll_stream(&mut self) {
        if !self.is_streaming {
            return;
        }

        if self.stream_rx.is_none() {
            return;
        }

        let mut received = false;

        loop {
            let token = {
                let rx = self.stream_rx.as_mut().unwrap();
                match rx.try_recv() {
                    Ok(token) => Some(token),
                    Err(mpsc::error::TryRecvError::Empty) => None,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        self.is_streaming = false;
                        self.stream_rx = None;

                        if let Some(channel) = &self.channel {
                            let session = self.current_session();
                            if session.messages.len() >= 2 {
                                let user_msg = &session.messages[session.messages.len() - 2];
                                let ai_msg = session.messages.last().unwrap();
                                let session_msg = SessionMessage {
                                    session_id: session.id,
                                    provider: session.provider_name.clone(),
                                    model: session.model.clone(),
                                    user_message: user_msg.clone(),
                                    ai_message: ai_msg.clone(),
                                };
                                let ch = Arc::clone(channel);
                                tokio::spawn(async move {
                                    ch.publish(session_msg.session_id, session_msg).await;
                                });
                            }
                        }
                        self.pending_user_msg = None;
                        None
                    }
                }
            };

            match token {
                Some(text) => {
                    if let Some(last_msg) = self.current_session_mut().messages.last_mut() {
                        last_msg.content.push_str(&text);
                        self.needs_draw = true;
                    }
                    received = true;
                }
                None => break,
            }
        }

        if received && self.scroll_offset == 0 {
            self.scroll_offset = 0;
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let pos = byte_to_char_boundary(&self.input, self.cursor_byte);
        self.input.insert(pos, c);
        self.cursor_byte = char_to_byte_offset(&self.input, self.input[..pos].chars().count() + 1);
        self.needs_draw = true;
    }

    pub fn insert_str(&mut self, s: &str) {
        let pos = byte_to_char_boundary(&self.input, self.cursor_byte);
        self.input.insert_str(pos, s);
        self.cursor_byte = pos + s.len();
        self.needs_draw = true;
    }

    pub fn backspace(&mut self) {
        if self.cursor_byte > 0 {
            let prev = prev_char_boundary(&self.input, self.cursor_byte);
            let _ = self.input.drain(prev..self.cursor_byte);
            self.cursor_byte = prev;
            self.needs_draw = true;
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_byte < self.input.len() {
            let next = next_char_boundary(&self.input, self.cursor_byte);
            let _ = self.input.drain(self.cursor_byte..next);
            self.needs_draw = true;
        }
    }

    pub fn delete_word(&mut self) {
        if self.cursor_byte == 0 {
            return;
        }
        let before = &self.input[..self.cursor_byte];
        if let Some(pos) = before.rfind(' ') {
            let remove_start = char_to_byte_offset(&self.input, before[pos..].chars().count() + pos);
            self.input.drain(remove_start..self.cursor_byte);
            self.cursor_byte = remove_start;
        } else {
            self.input.drain(..self.cursor_byte);
            self.cursor_byte = 0;
        }
        self.needs_draw = true;
    }

    pub fn cursor_left(&mut self) {
        self.cursor_byte = prev_char_boundary(&self.input, self.cursor_byte);
        self.needs_draw = true;
    }

    pub fn cursor_right(&mut self) {
        self.cursor_byte = next_char_boundary(&self.input, self.cursor_byte);
        self.needs_draw = true;
    }

    pub fn cursor_home(&mut self) {
        self.cursor_byte = 0;
        self.needs_draw = true;
    }

    pub fn cursor_end(&mut self) {
        self.cursor_byte = self.input.len();
        self.needs_draw = true;
    }

    pub fn cursor_up(&mut self) {
        if self.input.is_empty() {
            return;
        }
        self.cursor_byte = byte_to_char_boundary(&self.input, self.cursor_byte);
        let prev_nl = self.input[..self.cursor_byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
        if prev_nl == 0 {
            return;
        }
        let col = self.input[prev_nl..self.cursor_byte].width();
        let prev_start = self.input[..prev_nl.saturating_sub(1)].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prev_line_end = prev_nl.saturating_sub(1);
        let prev_line = &self.input[prev_start..prev_line_end];
        let target = nth_width_byte(prev_line, col).saturating_add(prev_start);
        self.cursor_byte = target.min(prev_line_end);
        self.needs_draw = true;
    }

    pub fn cursor_down(&mut self) {
        if self.input.is_empty() {
            return;
        }
        self.cursor_byte = byte_to_char_boundary(&self.input, self.cursor_byte);
        let next_nl = self.input[self.cursor_byte..].find('\n').map(|i| self.cursor_byte + i + 1);
        let next_start = match next_nl {
            Some(pos) => pos,
            None => return,
        };
        let line_start = self.input[..self.cursor_byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = self.input[line_start..self.cursor_byte].width();
        let next_line_end = self.input[next_start..].find('\n').map(|i| next_start + i).unwrap_or(self.input.len());
        let next_line = &self.input[next_start..next_line_end];
        let target = nth_width_byte(next_line, col).saturating_add(next_start);
        self.cursor_byte = target.min(next_line_end);
        self.needs_draw = true;
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.at_bottom = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.needs_draw = true;
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
        self.needs_draw = true;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.at_bottom = true;
        self.needs_draw = true;
    }
}

fn byte_to_char_boundary(s: &str, byte_pos: usize) -> usize {
    if byte_pos >= s.len() {
        return s.len();
    }
    let mut p = byte_pos;
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut p = pos - 1;
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

fn next_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }
    let mut p = pos + 1;
    while p < s.len() && !s.is_char_boundary(p) {
        p += 1;
    }
    if p > s.len() {
        s.len()
    } else {
        p
    }
}

fn char_to_byte_offset(s: &str, char_count: usize) -> usize {
    s.char_indices()
        .nth(char_count)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn nth_width_byte(s: &str, target_width: usize) -> usize {
    let mut w = 0;
    for (i, c) in s.char_indices() {
        let cw = c.width().unwrap_or(0);
        if w + cw > target_width {
            return i;
        }
        w += cw;
    }
    s.len()
}
