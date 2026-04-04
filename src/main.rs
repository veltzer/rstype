use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;
use serde::{Deserialize, Serialize};

const TARGET_TEXT: &str = "The quick brown fox jumps over the lazy dog";

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TypingMode {
    /// Cursor advances even on wrong key (errors are shown in red).
    Forward,
    /// Cursor stays on wrong key until the correct one is pressed.
    Stop,
}

impl TypingMode {
    fn label(self) -> &'static str {
        match self {
            TypingMode::Forward => "Forward — wrong key advances cursor (marked red)",
            TypingMode::Stop => "Stop    — wrong key blocks cursor until corrected",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    mode: TypingMode,
}

impl Default for Config {
    fn default() -> Self {
        Self { mode: TypingMode::Forward }
    }
}

fn config_path() -> PathBuf {
    let mut p = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    p.push(".config");
    p.push("rstype.toml");
    p
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn load_config() -> Config {
    let path = config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(cfg: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(s) = toml::to_string(cfg) {
        let _ = fs::write(&path, s);
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum Screen {
    Typing,
    Config,
}

struct App {
    // config (persisted)
    config: Config,

    // typing session
    target: Vec<char>,
    /// Correctly / wrongly typed chars so far (only advances in Forward; only
    /// on correct key in Stop).
    typed: Vec<char>,
    /// Cursor position (== typed.len() in Forward mode; may differ in Stop).
    cursor: usize,
    /// Total wrong keypresses (all modes).
    errors: usize,
    /// True for one render cycle after a wrong key in Stop mode (visual flash).
    error_flash: bool,

    typing_state: TypingState,
    start_time: Option<Instant>,
    wpm: f64,

    // navigation
    screen: Screen,
    /// Selected index on the config screen.
    config_cursor: usize,
}

#[derive(PartialEq)]
enum TypingState {
    Waiting,
    Typing,
    Done,
}

impl App {
    fn new(config: Config) -> Self {
        Self {
            config,
            target: TARGET_TEXT.chars().collect(),
            typed: Vec::new(),
            cursor: 0,
            errors: 0,
            error_flash: false,
            typing_state: TypingState::Waiting,
            start_time: None,
            wpm: 0.0,
            screen: Screen::Typing,
            config_cursor: 0,
        }
    }

    fn restart(&mut self) {
        let cfg = self.config.clone();
        *self = App::new(cfg);
    }

    /// Returns true if the app should quit.
    fn on_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // Global Ctrl shortcuts — work from any screen
        if ctrl {
            match key.code {
                KeyCode::Char('e') => return true,
                KeyCode::Char('t') => {
                    self.screen = Screen::Typing;
                    return false;
                }
                KeyCode::Char('c') => {
                    self.open_config();
                    return false;
                }
                _ => {}
            }
        }

        // Esc goes back to train screen (from config), or quits if already on train
        if key.code == KeyCode::Esc {
            match self.screen {
                Screen::Config => {
                    self.screen = Screen::Typing;
                    return false;
                }
                Screen::Typing => return true,
            }
        }

        match self.screen {
            Screen::Config => self.on_key_config(key),
            Screen::Typing => self.on_key_typing(key),
        }
        false
    }

    fn on_key_config(&mut self, key: KeyEvent) {
        const MODES: [TypingMode; 2] = [TypingMode::Forward, TypingMode::Stop];
        match key.code {
            KeyCode::Up => {
                if self.config_cursor > 0 {
                    self.config_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if self.config_cursor + 1 < MODES.len() {
                    self.config_cursor += 1;
                }
            }
            KeyCode::Enter => {
                self.config.mode = MODES[self.config_cursor];
                save_config(&self.config);
                self.screen = Screen::Typing;
            }
            _ => {}
        }
    }

    fn on_key_typing(&mut self, key: KeyEvent) {
        match self.typing_state {
            TypingState::Done => {
                match key.code {
                    KeyCode::Char('r') | KeyCode::Enter => self.restart(),
                    _ => {}
                }
            }
            TypingState::Waiting | TypingState::Typing => {
                match key.code {
                    KeyCode::Backspace => {
                        if self.cursor > 0 {
                            self.cursor -= 1;
                            self.typed.pop();
                        }
                    }
                    KeyCode::Char(ch) => {
                        self.error_flash = false;

                        if self.typing_state == TypingState::Waiting {
                            self.typing_state = TypingState::Typing;
                            self.start_time = Some(Instant::now());
                        }

                        let expected = self.target[self.cursor];

                        match self.config.mode {
                            TypingMode::Forward => {
                                self.typed.push(ch);
                                self.cursor += 1;
                                if ch != expected {
                                    self.errors += 1;
                                }
                            }
                            TypingMode::Stop => {
                                if ch == expected {
                                    self.typed.push(ch);
                                    self.cursor += 1;
                                } else {
                                    self.errors += 1;
                                    self.error_flash = true;
                                }
                            }
                        }

                        if self.cursor == self.target.len() {
                            let elapsed = self.start_time.unwrap().elapsed();
                            let minutes = elapsed.as_secs_f64() / 60.0;
                            let words = self.target.len() as f64 / 5.0;
                            self.wpm = words / minutes;
                            self.typing_state = TypingState::Done;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn open_config(&mut self) {
        // Pre-select the current mode
        self.config_cursor = match self.config.mode {
            TypingMode::Forward => 0,
            TypingMode::Stop => 1,
        };
        self.screen = Screen::Config;
    }

    fn accuracy(&self) -> f64 {
        let total_keys = self.typed.len() + self.errors;
        if total_keys == 0 {
            return 100.0;
        }
        let correct = self
            .typed
            .iter()
            .zip(self.target.iter())
            .filter(|(t, r)| t == r)
            .count();
        correct as f64 / total_keys as f64 * 100.0
    }
}

// ── Layout helpers ────────────────────────────────────────────────────────────

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &App) -> io::Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();
        frame.render_widget(Block::default().style(Style::default().bg(Color::Black)), area);

        // Reserve top row for toolbar
        let toolbar_rect = Rect::new(area.x, area.y, area.width, 1);
        let body_rect = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));

        render_toolbar(frame, toolbar_rect, app);

        match app.screen {
            Screen::Config => render_config(frame, body_rect, app),
            Screen::Typing => match app.typing_state {
                TypingState::Done => render_done(frame, body_rect, app),
                _ => render_typing(frame, body_rect, app),
            },
        }
    })?;
    Ok(())
}

fn render_toolbar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let active_style     = Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD);
    let normal_style     = Style::default().fg(Color::Black).bg(Color::White);
    let key_style        = Style::default().fg(Color::Blue).bg(Color::White).add_modifier(Modifier::BOLD);
    let active_key_style = Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD);
    let sep_style        = Style::default().fg(Color::DarkGray).bg(Color::White);

    let is_train  = app.screen == Screen::Typing;
    let is_config = app.screen == Screen::Config;

    // Helper: builds spans for one toolbar button like "^T Train"
    let mut spans = vec![Span::styled("  ", normal_style)];

    for (label, shortcut, active) in [
        ("Train",  'T', is_train),
        ("Config", 'C', is_config),
        ("Exit",   'E', false),
    ] {
        let (ks, rs) = if active { (active_key_style, active_style) } else { (key_style, normal_style) };
        spans.push(Span::styled("^", ks));
        spans.push(Span::styled(shortcut.to_string(), ks));
        spans.push(Span::styled(format!(" {label}"), rs));
        spans.push(Span::styled("  ", sep_style));
    }

    // Fill remainder
    spans.push(Span::styled(" ".repeat(area.width as usize), normal_style));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_typing(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = Vec::new();

    for (i, &ch) in app.target.iter().enumerate() {
        let span = if i < app.cursor {
            // Already typed
            if app.typed[i] == ch {
                Span::styled(ch.to_string(), Style::default().fg(Color::Green))
            } else {
                // Forward mode wrong char
                Span::styled(
                    ch.to_string(),
                    Style::default().fg(Color::Red).add_modifier(Modifier::UNDERLINED),
                )
            }
        } else if i == app.cursor {
            // Current cursor
            let bg = if app.error_flash { Color::Red } else { Color::White };
            Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(Color::Black)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(ch.to_string(), Style::default().fg(Color::DarkGray))
        };
        spans.push(span);
    }

    let mode_label = match app.config.mode {
        TypingMode::Forward => "forward",
        TypingMode::Stop => "stop",
    };

    let title = match app.typing_state {
        TypingState::Waiting => format!(" rstype [{mode_label}] — press any key to start, C for config "),
        _ => format!(" rstype [{mode_label}] — typing… (Backspace to correct) "),
    };

    let text_width = (app.target.len() as u16 + 4).min(area.width);
    let box_rect = centered_rect(text_width, 5, area);

    let paragraph = Paragraph::new(vec![Line::from(""), Line::from(spans)])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.as_str())
                .title_alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, box_rect);

    // Progress bar
    let progress_pct = app.cursor as f64 / app.target.len() as f64;
    let bar_width = box_rect.width.saturating_sub(2) as f64;
    let filled = (bar_width * progress_pct) as usize;
    let empty = bar_width as usize - filled;
    let bar_text = format!(
        "[{}{}] {:.0}%  errors: {}",
        "█".repeat(filled),
        "░".repeat(empty),
        progress_pct * 100.0,
        app.errors
    );

    let bar_rect = Rect::new(box_rect.x, box_rect.y + box_rect.height + 1, box_rect.width, 1);
    if bar_rect.bottom() <= area.bottom() {
        frame.render_widget(
            Paragraph::new(bar_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow)),
            bar_rect,
        );
    }
}

fn render_done(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let accuracy = app.accuracy();
    let acc_color = if accuracy >= 95.0 {
        Color::Green
    } else if accuracy >= 80.0 {
        Color::Yellow
    } else {
        Color::Red
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Speed:    "),
            Span::styled(
                format!("{:.1} WPM", app.wpm),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Accuracy: "),
            Span::styled(
                format!("{:.1}%", accuracy),
                Style::default().fg(acc_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Errors:   "),
            Span::styled(
                format!("{}", app.errors),
                Style::default()
                    .fg(if app.errors == 0 { Color::Green } else { Color::Red })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let result_rect = centered_rect(52, 8, area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Results ")
                .title_alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow)),
        ),
        result_rect,
    );
}

fn render_config(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    const MODES: [TypingMode; 2] = [TypingMode::Forward, TypingMode::Stop];

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Select typing mode:",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
    ];

    for (i, mode) in MODES.iter().enumerate() {
        let selected = i == app.config_cursor;
        let active = *mode == app.config.mode;

        let prefix = if selected { "▶ " } else { "  " };
        let suffix = if active { "  ✓" } else { "" };
        let label = format!("{}{}{}", prefix, mode.label(), suffix);

        let style = if selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(Span::styled(label, style)));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  ↑/↓ to move   Enter to save   ^T back to train",
        Style::default().fg(Color::DarkGray),
    )));

    let box_rect = centered_rect(60, (lines.len() as u16) + 2, area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Config — ~/.config/rstype.toml ")
                .title_alignment(Alignment::Center)
                .style(Style::default().fg(Color::Magenta)),
        ),
        box_rect,
    );
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let config = load_config();
    let mut app = App::new(config);

    loop {
        render(&mut terminal, &app)?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if app.on_key(key) {
                    break;
                }
                // Clear flash after it's been rendered once
                app.error_flash = false;
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
