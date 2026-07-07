use std::collections::HashMap;
use std::sync::Arc;

use pulldown_cmark::{CodeBlockKind, Event, Options, Tag, TagEnd};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};

use textwrap::Options as WrapOpts;
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Picker};
use crate::backend::ChatBackend;
use crate::models::Role;
use crate::theme::Theme;

pub fn render(
    f: &mut Frame,
    app: &mut App,
    theme: &Theme,
) {
    let area = f.area();
    let margin_block = Block::default()
        .padding(Padding::uniform(1))
        .style(Style::default().bg(theme.bg));
    let area = margin_block.inner(area);
    f.render_widget(margin_block, f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(5),
        ])
        .split(area);

    let chat_width = main_chunks[1].width;

    let session = app.current_session();
    let model_count = app.backends.get(&session.provider_name).map(|b| b.models().len()).unwrap_or(0);
    let model_idx = app.backends
        .get(&session.provider_name)
        .and_then(|b| b.models().iter().position(|m| m == &session.model))
        .map(|i| i + 1)
        .unwrap_or(0);

    let muted = Style::default().fg(theme.status_text).bg(theme.bg);
    let accent = Style::default().fg(theme.user_text).bg(theme.bg);

    let streaming_mark = if app.is_streaming {
        Span::styled(" ● streaming ", Style::default().fg(theme.system_text).bg(theme.bg).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("", muted)
    };

    let status = Line::from(vec![
        Span::styled(" >", accent),
        Span::styled(&session.provider_name, Style::default().fg(theme.input_text).bg(theme.bg)),
        Span::styled("/", muted),
        Span::styled(&session.model, Style::default().fg(theme.input_text).bg(theme.bg)),
        Span::styled(format!(" ({}/{})", model_idx, model_count), muted),
        Span::styled(format!("  session {}/{}", app.current_session + 1, app.sessions.len()), muted),
        streaming_mark,
    ]);
    f.render_widget(Paragraph::new(status).style(Style::default().bg(theme.bg)), main_chunks[0]);

    if app.show_help {
        render_help(f, main_chunks[1], &app.backends, theme);
    } else {
        render_chat_area(f, main_chunks[1], app, chat_width, theme);
    }

    render_input(f, main_chunks[2], &app.input, app.cursor_byte, app.is_streaming, theme, &mut app.input_scroll);

    if let Some(p) = &app.picker {
        render_picker(f, f.area(), p, theme);
    }
}

fn render_help(
    f: &mut Frame,
    area: Rect,
    backends: &HashMap<String, Arc<dyn ChatBackend>>,
    theme: &Theme,
) {
    let muted = Style::default().fg(theme.hint_text);
    let accent = Style::default().fg(theme.user_text);
    let green = Style::default().fg(theme.ai_text);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("─".repeat(area.width as usize), muted),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" Commands", Style::default().fg(theme.system_text).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));

    let commands: &[(&str, &str)] = &[
        ("/m, /model <name>", "switch model"),
        ("/m, /models", "pick a model"),
        ("/p, /provider <name>", "switch provider"),
        ("/p, /providers", "pick a provider"),
        ("/t, /theme <name>", "switch theme"),
        ("/t, /themes", "pick a theme"),
        ("/n, /new", "new session"),
        ("/s, /sessions", "pick a session"),
        ("/q, /quit", "exit"),
        ("/h, /help", "toggle this panel"),
    ];
    for (cmd, desc) in commands {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<24}", cmd), accent),
            Span::styled(desc.to_string(), muted),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("─".repeat(area.width as usize), muted)]));
    lines.push(Line::from(vec![
        Span::styled(" Keys", Style::default().fg(theme.system_text).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    let keys: &[(&str, &str)] = &[
        ("Enter", "send message or execute /command"),
        ("Ctrl+A / E", "jump to start / end of input"),
        ("Ctrl+W", "delete word before cursor"),
        ("Delete", "delete character after cursor"),
        ("← →", "move cursor in input"),
        ("Home / End", "jump input cursor / scroll to bottom"),
        ("↑ ↓", "scroll chat"),
        ("PgUp/PgDn", "scroll chat faster"),
        ("Tab", "next session"),
        ("Esc", "close help or picker"),
        ("mouse scroll", "scroll chat"),
    ];
    for (key, desc) in keys {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<24}", key), accent),
            Span::styled(desc.to_string(), muted),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled("─".repeat(area.width as usize), muted)]));
    lines.push(Line::from(vec![
        Span::styled(" Providers & Models", Style::default().fg(theme.system_text).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));

    let mut sorted: Vec<(&String, &Arc<dyn ChatBackend>)> = backends.iter().collect();
    sorted.sort_by_key(|(name, _)| *name);
    for (name, backend) in sorted {
        let models = backend.models();
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<12}", name), accent.add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} models", models.len()), muted),
        ]));
        for m in models.iter().take(8) {
            lines.push(Line::from(vec![Span::styled(format!("    - {}", m), green)]));
        }
        if models.len() > 8 {
            lines.push(Line::from(vec![Span::styled(format!("    ... and {} more", models.len() - 8), muted)]));
        }
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![Span::styled("─".repeat(area.width as usize), muted)]));

    let p = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(theme.bg))
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_chat_area(
    f: &mut Frame,
    area: Rect,
    app: &mut App,
    chat_width: u16,
    theme: &Theme,
) {
    if area.height == 0 || chat_width < 4 {
        return;
    }

    let width = chat_width.saturating_sub(2) as usize;
    let (msg_count, last_ai_len) = {
        let msgs = &app.current_session().messages;
        let count = msgs.len();
        let ai_len = msgs.last()
            .filter(|m| matches!(m.role, Role::Assistant))
            .map(|m| m.content.len())
            .unwrap_or(0);
        (count, ai_len)
    };

    let rebuild = app.cache_msg_count != msg_count
        || app.cache_last_ai_len != last_ai_len
        || app.cache_width != chat_width;

    let mut lines: Vec<Line> = if rebuild {
        let messages = &app.current_session().messages;
        let new_lines = build_chat_lines(messages, width, theme);
        new_lines
    } else {
        app.rendered_cache.clone()
    };

    if rebuild {
        app.rendered_cache = lines.clone();
        app.cache_msg_count = msg_count;
        app.cache_last_ai_len = last_ai_len;
        app.cache_width = chat_width;
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Start typing to chat — Enter to send, /q to quit",
            Style::default().fg(theme.hint_text),
        )));
    }

    let total_lines = lines.len().max(1);
    let max_scroll = total_lines.saturating_sub(area.height as usize);
    let mut scroll_offset = app.scroll_offset;
    if app.at_bottom {
        scroll_offset = max_scroll;
        app.scroll_offset = max_scroll;
    }
    let offset = scroll_offset.min(max_scroll);
    let end = (offset + area.height as usize).min(lines.len());
    let visible: Vec<Line> = if offset < end {
        lines[offset..end].to_vec()
    } else {
        vec![]
    };

    let p = Paragraph::new(Text::from(visible))
        .style(Style::default().bg(theme.bg));
    f.render_widget(p, area);
}

fn build_chat_lines(
    messages: &[crate::models::Message],
    width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut new_lines: Vec<Line> = Vec::new();
    for msg in messages {
        let color = match msg.role {
            Role::User => theme.user_text,
            Role::Assistant => theme.ai_text,
            Role::System => theme.system_text,
        };

        if matches!(msg.role, Role::Assistant) {
            new_lines.push(Line::from(vec![Span::styled(
                "  AI".to_string(),
                Style::default().fg(theme.ai_text).add_modifier(Modifier::BOLD),
            )]));
            let md_lines = markdown_to_lines(&msg.content, theme, color, width);
            for line in md_lines {
                let mut spans: Vec<Span> = vec![Span::styled("  ".to_string(), Style::default().fg(color))];
                spans.extend(line.spans.iter().map(|s| Span::styled(s.content.clone(), s.style)));
                new_lines.push(Line::from(spans));
            }
            new_lines.push(Line::from(""));
        } else {
            let prefix = if matches!(msg.role, Role::User) { "  You" } else { "  System" };
            new_lines.push(Line::from(vec![Span::styled(
                prefix.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )]));
            let content = crate::latex::latex_to_unicode(&msg.content);
            for wrapped in textwrap::wrap(&content, width) {
                new_lines.push(Line::from(vec![Span::styled(
                    format!("  {}", wrapped),
                    Style::default().fg(color),
                )]));
            }
            new_lines.push(Line::from(""));
        }
    }
    new_lines
}

fn markdown_to_lines(
    content: &str,
    theme: &Theme,
    base_color: ratatui::style::Color,
    width: usize,
) -> Vec<Line<'static>> {
    let content = crate::latex::latex_to_unicode(content);
    let mut lines: Vec<Line> = Vec::new();
    let mut current_text = String::new();
    let mut current_spans: Vec<Span> = Vec::new();
    let mut modifiers: Vec<Modifier> = Vec::new();
    let mut in_code_block = false;

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = pulldown_cmark::Parser::new_ext(&content, opts);

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::CodeBlock(CodeBlockKind::Fenced(_)) => {
                    flush_text(&mut current_text, &mut current_spans, &modifiers, in_code_block, base_color, theme);
                    in_code_block = true;
                }
                Tag::Heading { .. } => {
                    modifiers.clear();
                    modifiers.push(Modifier::BOLD);
                }
                Tag::Strong => modifiers.push(Modifier::BOLD),
                Tag::Emphasis => modifiers.push(Modifier::ITALIC),
                Tag::Strikethrough => modifiers.push(Modifier::CROSSED_OUT),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::CodeBlock => {
                    flush_text(&mut current_text, &mut current_spans, &modifiers, in_code_block, base_color, theme);
                    if !current_spans.is_empty() {
                        emit_spans(&mut lines, std::mem::take(&mut current_spans), width.max(10), in_code_block, base_color, theme);
                    }
                    lines.push(Line::from(""));
                    in_code_block = false;
                    modifiers.clear();
                }
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::List(_) => {
                    flush_text(&mut current_text, &mut current_spans, &modifiers, in_code_block, base_color, theme);
                    if !current_spans.is_empty() {
                        emit_spans(&mut lines, std::mem::take(&mut current_spans), width.max(10), in_code_block, base_color, theme);
                    }
                    lines.push(Line::from(""));
                    modifiers.clear();
                }
                TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough => {
                    flush_text(&mut current_text, &mut current_spans, &modifiers, in_code_block, base_color, theme);
                    modifiers.pop();
                }
                _ => {}
            },
            Event::Text(text) => {
                current_text.push_str(&text);
            }
            Event::Code(code) => {
                flush_text(&mut current_text, &mut current_spans, &modifiers, in_code_block, base_color, theme);
                current_spans.push(Span::styled(code.to_string(), Style::default().fg(theme.system_text)));
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_text(&mut current_text, &mut current_spans, &modifiers, in_code_block, base_color, theme);
            }
            _ => {}
        }
    }

    flush_text(&mut current_text, &mut current_spans, &modifiers, in_code_block, base_color, theme);
    if !current_spans.is_empty() {
        emit_spans(&mut lines, std::mem::take(&mut current_spans), width.max(10), in_code_block, base_color, theme);
    }

    lines
}

fn eof_style(in_code_block: bool, base_color: ratatui::style::Color, theme: &Theme) -> ratatui::style::Style {
    if in_code_block {
        Style::default().fg(theme.system_text)
    } else {
        Style::default().fg(base_color)
    }
}

fn flush_text(text: &mut String, spans: &mut Vec<Span>, mods: &[Modifier], in_code_block: bool, base_color: ratatui::style::Color, theme: &Theme) {
    if text.is_empty() {
        return;
    }
    let style = eof_style(in_code_block, base_color, theme)
        .add_modifier(mods.iter().fold(Modifier::empty(), |a, m| a | *m));
    spans.push(Span::styled(std::mem::take(text), style));
}

fn emit_spans(lines: &mut Vec<Line>, spans: Vec<Span>, w: usize, in_code_block: bool, base_color: ratatui::style::Color, theme: &Theme) {
    let raw: String = spans.iter().map(|s| s.content.as_ref()).collect::<Vec<_>>().join("");
    let wrap_opts = if in_code_block {
        WrapOpts::new(w).break_words(true)
    } else {
        WrapOpts::new(w)
    };
    for chunk in textwrap::wrap(&raw, wrap_opts) {
        let chunk_s = chunk.to_string();
        let mut chunk_spans = Vec::new();
        let mut remaining = chunk_s.as_str();
        for span in &spans {
            if remaining.is_empty() { break; }
            let sc = span.content.as_ref();
            if remaining.starts_with(sc) {
                chunk_spans.push(Span::styled(sc.to_string(), span.style));
                remaining = &remaining[sc.len()..];
            }
            let overlap = common_prefix_len(remaining, sc);
            if overlap > 0 {
                chunk_spans.push(Span::styled(remaining[..overlap].to_string(), span.style));
                remaining = &remaining[overlap..];
            }
        }
        if !remaining.is_empty() {
            chunk_spans.push(Span::styled(remaining.to_string(), eof_style(in_code_block, base_color, theme)));
        }
        lines.push(Line::from(chunk_spans));
    }
}

fn common_prefix_len(a: &str, b: &str) -> usize {
    a.char_indices()
        .zip(b.chars())
        .take_while(|((_, ac), bc)| ac == bc)
        .last()
        .map(|((i, c), _)| i + c.len_utf8())
        .unwrap_or(0)
}

fn render_input(
    f: &mut Frame,
    area: Rect,
    input: &str,
    cursor_byte: usize,
    is_streaming: bool,
    theme: &Theme,
    _input_scroll: &mut usize,
) {
    let border_color = if is_streaming {
        theme.system_text
    } else {
        theme.input_border
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg))
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if input.is_empty() {
        let hint = Line::from(vec![
            Span::styled("Type a message, or /command...    ", Style::default().fg(theme.hint_text)),
            Span::styled("Enter", Style::default().fg(theme.user_text)),
            Span::styled(": send    ", Style::default().fg(theme.hint_text)),
            Span::styled("/h", Style::default().fg(theme.user_text)),
            Span::styled(": help    ", Style::default().fg(theme.hint_text)),
            Span::styled("/q", Style::default().fg(theme.user_text)),
            Span::styled(": quit", Style::default().fg(theme.hint_text)),
        ]);
        f.render_widget(Paragraph::new(hint).style(Style::default().bg(theme.bg)), inner);
        f.set_cursor_position((inner.x, inner.y));
    } else {
        let mut cursor_byte = cursor_byte.min(input.len());
        while cursor_byte > 0 && !input.is_char_boundary(cursor_byte) {
            cursor_byte -= 1;
        }
        let mut text_lines: Vec<Line> = Vec::new();
        let mut cursor_row = 0usize;
        let mut cursor_col = 0u16;
        let mut byte_offset = 0usize;

        for (row, raw) in input.split('\n').enumerate() {
            let line_end = byte_offset + raw.len();
            let nl_len = if line_end < input.len() { 1 } else { 0 };

            if cursor_byte >= byte_offset && cursor_byte <= line_end + nl_len {
                cursor_row = row;
                let local = cursor_byte.saturating_sub(byte_offset);
                let real = local.min(raw.len());
                let before_local = &raw[..real];

                cursor_col = before_local.width() as u16;

                let cursor_in_middle = local < raw.len();
                if cursor_in_middle {
                    let mut local_byte = local;
                    while local_byte > 0 && !raw.is_char_boundary(local_byte) {
                        local_byte -= 1;
                    }
                    let ch = raw[local_byte..].chars().next().unwrap_or(' ');
                    let after = &raw[local_byte + ch.len_utf8()..];
                    let before = &raw[..local_byte];
                    text_lines.push(Line::from(vec![
                        Span::styled(before.to_string(), Style::default().fg(theme.input_text)),
                        Span::styled(ch.to_string(), Style::default().fg(theme.bg).bg(theme.input_text)),
                        Span::styled(after.to_string(), Style::default().fg(theme.input_text)),
                    ]));
                } else {
                    text_lines.push(Line::from(vec![
                        Span::styled(raw.to_string(), Style::default().fg(theme.input_text)),
                    ]));
                }
            } else {
                text_lines.push(Line::from(Span::styled(raw.to_string(), Style::default().fg(theme.input_text))));
            }

            byte_offset = line_end + nl_len;
        }

        let inner_rows = inner.height as usize;
        let scroll = *_input_scroll;

        if cursor_row < scroll {
            *_input_scroll = cursor_row;
        } else if cursor_row >= scroll + inner_rows {
            *_input_scroll = cursor_row.saturating_sub(inner_rows - 1);
        }

        let updated_scroll = (*_input_scroll).min(text_lines.len().saturating_sub(1));
        let end = (updated_scroll + inner_rows).min(text_lines.len());
        let visible: Vec<Line> = if updated_scroll < text_lines.len() {
            text_lines[updated_scroll..end].to_vec()
        } else {
            text_lines.clone()
        };
        let cursor_y = inner.y + (cursor_row.saturating_sub(updated_scroll)) as u16;

        f.render_widget(
            Paragraph::new(Text::from(visible))
                .style(Style::default().bg(theme.bg))
                .wrap(Wrap { trim: false }),
            inner,
        );
        f.set_cursor_position((inner.x + cursor_col, cursor_y));
    }
}

fn render_picker(f: &mut Frame, area: Rect, picker: &Picker, theme: &Theme) {
    let max_items = 12;
    let list_height = picker.items.len().min(max_items) as u16;
    let panel_height = list_height + 4;
    let panel_width = 52;

    let x = area.x + (area.width.saturating_sub(panel_width)) / 2;
    let y = area.y + (area.height.saturating_sub(panel_height)) / 2;
    let picker_area = Rect::new(x, y, panel_width, panel_height);

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg))
        .border_style(Style::default().fg(theme.user_text));
    let inner = block.inner(picker_area);
    f.render_widget(block, picker_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        picker.title.clone(),
        Style::default().fg(theme.user_text).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let start = if picker.selected >= max_items { picker.selected - max_items + 1 } else { 0 };
    let end = (start + max_items).min(picker.items.len());

    for (i, item) in picker.items[start..end].iter().enumerate() {
        let idx = start + i;
        if idx == picker.selected {
            lines.push(Line::from(vec![Span::styled(
                format!(" > {}", item),
                Style::default().fg(theme.bg).bg(theme.user_text).add_modifier(Modifier::BOLD),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                format!("   {}", item),
                Style::default().fg(theme.ai_text),
            )]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" Enter: select  ", Style::default().fg(theme.status_text)),
        Span::styled("Esc: cancel  ", Style::default().fg(theme.status_text)),
        Span::styled("↑↓: navigate", Style::default().fg(theme.status_text)),
    ]));

    f.render_widget(
        Paragraph::new(Text::from(lines)).style(Style::default().bg(theme.bg)),
        inner,
    );
}
