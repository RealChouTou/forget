# forget

Terminal chat client for multiple LLM providers.

## Install

```bash
cargo build --release
```

## Setup

First run creates `~/.config/forget/config.toml`. Edit it:

```toml
theme = "dark"
default_provider = "deepseek"
default_model = "deepseek-chat"

[deepseek]
api_key = "sk-your-key"
base_url = "https://api.deepseek.com/v1"

[ollama]
base_url = "http://localhost:11434"
```

API keys can also be set via environment variables:

| Provider   | Variable            |
|-----------|---------------------|
| openai    | `OPENAI_API_KEY`    |
| deepseek  | `DEEPSEEK_API_KEY`  |
| qwen      | `QWEN_API_KEY`      |
| anthropic | `ANTHROPIC_API_KEY` |

Ollama needs no API key. Placeholder values like `sk-...` are ignored.

Models are fetched from each provider's API at startup. The `models` list in config is a fallback.

Last-used provider, model, and theme are persisted to `~/.config/forget/state.json`.

## Usage

```bash
cargo run
```

Type a message and press **Enter** to send. Lines starting with `/` are commands.

### Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `/model <name>` | `/m <name>` | Switch model |
| `/model` | `/m` | Open model picker |
| `/provider <name>` | `/p <name>` | Switch provider |
| `/provider` | `/p` | Open provider picker |
| `/theme <name>` | `/t <name>` | Switch theme |
| `/theme` | `/t` | Open theme picker |
| `/session <n>` | `/s <n>` | Switch to session n |
| `/session` | `/s` | Open session picker |
| `/new` | `/n` | New chat session |
| `/help` | `/h`, `/?` | Toggle help panel |
| `/quit` | `/q` | Quit |

Commands without an argument open a picker. Navigate with **↑↓**, select with **Enter**, cancel with **Esc**.

### Keys

| Key | Action |
|-----|--------|
| `Enter` | Send message or execute command |
| `Shift+Enter` | Insert newline |
| `Backspace` | Delete before cursor |
| `Delete` | Delete after cursor |
| `Ctrl+W` | Delete word before cursor |
| `Ctrl+A` / `Ctrl+E` | Cursor to start / end of input |
| `←` `→` | Move cursor in input |
| `Home` | Cursor to start of input |
| `End` | Cursor to end (or scroll chat to bottom if input empty) |
| `↑` `↓` | Scroll chat |
| `PgUp` `PgDn` | Scroll chat faster |
| `Tab` | Next session |
| `Esc` | Close help panel or picker |
| Mouse wheel | Scroll chat |

### Scrolling

Chat auto-follows AI output. Scroll up (keys or mouse) to pause auto-follow and read at your own pace. Press `End` or send a new message to resume.

## Providers

| Provider | API | Model Discovery |
|----------|-----|-----------------|
| openai | OpenAI-compatible `/v1` | `GET /v1/models` |
| deepseek | OpenAI-compatible `/v1` | `GET /v1/models` |
| qwen | OpenAI-compatible `/v1` | `GET /v1/models` |
| anthropic | Anthropic Messages API | config fallback |
| ollama | Ollama `/api/chat` | `GET /api/tags` |

## Themes

Four built-in themes: `dark` (default), `light`, `dracula`, `nord`. Set in config or switch at runtime with `/t`.

## Markdown

AI responses support **bold**, *italic*, ~~strikethrough~~, headings, lists, code blocks, and inline `code`. Long lines including code wrap to fit the terminal width. Code blocks wrap at character boundaries.

## Providers & Backends

Each provider is implemented via the `ChatBackend` trait (`src/backend/mod.rs`):

| File | Description |
|------|-------------|
| `openai_compat.rs` | OpenAI / DeepSeek / Qwen (OpenAI-compatible API) |
| `anthropic.rs` | Anthropic Messages API |
| `ollama.rs` | Ollama local API |

## Project Structure

```
src/
├── main.rs          Entry, event loop, key/mouse dispatch
├── app.rs           App state, sessions, commands, streaming, picker
├── tui.rs           Terminal rendering, markdown, help, input
├── config.rs        Config loading, env injection, auto-create
├── models.rs        Message, Role, SessionMessage
├── state.rs         Persist last provider/model/theme
├── theme.rs         Dark, Light, Dracula, Nord
├── channel.rs       Channel trait (extension point)
└── backend/
    ├── mod.rs       ChatBackend trait
    ├── openai_compat.rs
    ├── anthropic.rs
    └── ollama.rs
```

## License

MIT
