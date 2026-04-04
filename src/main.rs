use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
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

// ── History ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct Keystroke {
    typed: String,
    offset_ms: u64,
}

#[derive(Serialize)]
struct Session {
    timestamp: String,
    text: String,
    mode: String,
    wpm: f64,
    errors: usize,
    keystrokes: Vec<Keystroke>,
}

fn history_path() -> PathBuf {
    let mut p = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    p.push(".local");
    p.push("share");
    p.push("rstype");
    p.push("history.jsonl");
    p
}

fn save_session(session: &Session) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        if let Ok(line) = serde_json::to_string(session) {
            let _ = writeln!(file, "{}", line);
        }
    }
}

fn now_timestamp() -> String {
    // RFC 3339-ish without pulling in chrono: use std::time
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as ISO 8601 UTC manually
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400; // days since epoch
    // Compute year/month/day from days-since-epoch (simple Gregorian)
    let (year, month, day) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, h, m, s)
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        year += 1;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md { break; }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn keycode_to_w3c(code: KeyCode) -> String {
    match code {
        KeyCode::Char(' ')       => "Space".to_string(),
        KeyCode::Char(c)         => c.to_string(),
        KeyCode::Backspace       => "Backspace".to_string(),
        KeyCode::Enter           => "Enter".to_string(),
        KeyCode::Tab             => "Tab".to_string(),
        KeyCode::Esc             => "Escape".to_string(),
        KeyCode::Delete          => "Delete".to_string(),
        KeyCode::Insert          => "Insert".to_string(),
        KeyCode::Home            => "Home".to_string(),
        KeyCode::End             => "End".to_string(),
        KeyCode::PageUp          => "PageUp".to_string(),
        KeyCode::PageDown        => "PageDown".to_string(),
        KeyCode::Up              => "ArrowUp".to_string(),
        KeyCode::Down            => "ArrowDown".to_string(),
        KeyCode::Left            => "ArrowLeft".to_string(),
        KeyCode::Right           => "ArrowRight".to_string(),
        KeyCode::CapsLock        => "CapsLock".to_string(),
        KeyCode::F(n)            => format!("F{}", n),
        _                        => "Unidentified".to_string(),
    }
}

// ── Calendar helpers ──────────────────────────────────────────────────────────

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month_cal(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 31,
    }
}

/// Returns 0=Monday .. 6=Sunday for the first day of the given month.
fn first_weekday_of_month(year: i32, month: u32) -> u32 {
    // Tomohiko Sakamoto's algorithm (returns 0=Sun..6=Sat), convert to Mon=0..Sun=6
    let t = [0u32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let yu = y as u32;
    let dow = (yu + yu / 4 - yu / 100 + yu / 400 + t[(month - 1) as usize] + 1) % 7;
    (dow + 6) % 7 // Sun=0 -> Mon=0
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January", 2 => "February", 3 => "March",    4 => "April",
        5 => "May",     6 => "June",     7 => "July",      8 => "August",
        9 => "September", 10 => "October", 11 => "November", 12 => "December",
        _ => "?",
    }
}

/// (year, month) today from system clock.
fn today_ym() -> (i32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let (y, m, _) = days_to_ymd(days);
    (y as i32, m as u32)
}

/// Parse JSONL history and return a map of "YYYY-MM-DD" -> (session_count, avg_wpm).
fn load_history_stats() -> HashMap<String, (usize, f64)> {
    let mut map: HashMap<String, (usize, f64)> = HashMap::new();
    let path = history_path();
    let Ok(content) = fs::read_to_string(&path) else { return map; };
    for line in content.lines() {
        // Just extract "timestamp" and "wpm" fields cheaply
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if let (Some(ts), Some(wpm)) = (
                val.get("timestamp").and_then(|v| v.as_str()),
                val.get("wpm").and_then(|v| v.as_f64()),
            ) {
                let date_key = ts.get(..10).unwrap_or("").to_string();
                if date_key.len() == 10 {
                    let entry = map.entry(date_key).or_insert((0, 0.0));
                    entry.0 += 1;
                    entry.1 += wpm;
                }
            }
        }
    }
    // Convert wpm sum -> average
    for (_, v) in map.iter_mut() {
        v.1 /= v.0 as f64;
    }
    map
}

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
    #[serde(default = "default_min_cols")]
    min_cols: u16,
    #[serde(default = "default_min_rows")]
    min_rows: u16,
}

fn default_min_cols() -> u16 { 76 }
fn default_min_rows() -> u16 { 26 }

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: TypingMode::Forward,
            min_cols: default_min_cols(),
            min_rows: default_min_rows(),
        }
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
    Calendar,
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
    keystrokes: Vec<Keystroke>,

    // navigation
    screen: Screen,
    /// Selected index on the config screen.
    config_cursor: usize,
    // calendar
    calendar_year: i32,
    calendar_month: u32,
    calendar_stats: HashMap<String, (usize, f64)>,
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
            keystrokes: Vec::new(),
            screen: Screen::Typing,
            config_cursor: 0,
            calendar_year: 0,
            calendar_month: 1,
            calendar_stats: HashMap::new(),
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
                KeyCode::Char('c') => return true, // standard kill, always works
                KeyCode::Char('e') => return true,
                KeyCode::Char('t') => {
                    self.screen = Screen::Typing;
                    return false;
                }
                KeyCode::Char('g') => {
                    self.open_config();
                    return false;
                }
                KeyCode::Char('h') => {
                    self.open_calendar();
                    return false;
                }
                _ => {}
            }
        }

        // Esc goes back to train screen (from config/calendar), or quits if already on train
        if key.code == KeyCode::Esc {
            match self.screen {
                Screen::Config | Screen::Calendar => {
                    self.screen = Screen::Typing;
                    return false;
                }
                Screen::Typing => return true,
            }
        }

        match self.screen {
            Screen::Config => self.on_key_config(key),
            Screen::Typing => self.on_key_typing(key),
            Screen::Calendar => self.on_key_calendar(key),
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
                    KeyCode::Char('r') | KeyCode::Enter | KeyCode::Char(' ') => self.restart(),
                    _ => {}
                }
            }
            TypingState::Waiting | TypingState::Typing => {
                // Start timer on first keypress
                if self.typing_state == TypingState::Waiting {
                    if matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace) {
                        self.typing_state = TypingState::Typing;
                        self.start_time = Some(Instant::now());
                    }
                }

                // Record every keypress once the session has started
                if let Some(start) = self.start_time {
                    let offset_ms = start.elapsed().as_millis() as u64;
                    self.keystrokes.push(Keystroke {
                        typed: keycode_to_w3c(key.code),
                        offset_ms,
                    });
                }

                match key.code {
                    KeyCode::Backspace => {
                        if self.cursor > 0 {
                            self.cursor -= 1;
                            self.typed.pop();
                        }
                    }
                    KeyCode::Char(ch) => {
                        self.error_flash = false;

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

                            save_session(&Session {
                                timestamp: now_timestamp(),
                                text: self.target.iter().collect(),
                                mode: format!("{:?}", self.config.mode).to_lowercase(),
                                wpm: self.wpm,
                                errors: self.errors,
                                keystrokes: self.keystrokes.iter().map(|k| Keystroke {
                                    typed: k.typed.clone(),
                                    offset_ms: k.offset_ms,
                                }).collect(),
                            });
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

    fn open_calendar(&mut self) {
        let (y, m) = today_ym();
        self.calendar_year = y;
        self.calendar_month = m;
        self.calendar_stats = load_history_stats();
        self.screen = Screen::Calendar;
    }

    fn on_key_calendar(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left => {
                if self.calendar_month == 1 {
                    self.calendar_month = 12;
                    self.calendar_year -= 1;
                } else {
                    self.calendar_month -= 1;
                }
            }
            KeyCode::Right => {
                if self.calendar_month == 12 {
                    self.calendar_month = 1;
                    self.calendar_year += 1;
                } else {
                    self.calendar_month += 1;
                }
            }
            _ => {}
        }
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

        // Reserve top row for toolbar, bottom row for status bar
        let toolbar_rect  = Rect::new(area.x, area.y, area.width, 1);
        let statusbar_rect = Rect::new(area.x, area.y + area.height.saturating_sub(1), area.width, 1);
        let body_rect     = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(2));

        render_toolbar(frame, toolbar_rect, app);
        render_statusbar(frame, statusbar_rect, app);

        match app.screen {
            Screen::Config => render_config(frame, body_rect, app),
            Screen::Calendar => render_calendar(frame, body_rect, app),
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

    let is_train    = app.screen == Screen::Typing;
    let is_config   = app.screen == Screen::Config;
    let is_calendar = app.screen == Screen::Calendar;

    // Helper: builds spans for one toolbar button like "^T Train"
    let mut spans = vec![Span::styled("  ", normal_style)];

    for (label, shortcut, active) in [
        ("Train",   'T', is_train),
        ("Config",  'G', is_config),
        ("History", 'H', is_calendar),
        ("Exit",    'E', false),
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

fn render_statusbar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let style = Style::default().fg(Color::Black).bg(Color::White);
    let mode_label = match app.config.mode {
        TypingMode::Forward => "forward",
        TypingMode::Stop => "stop",
    };
    let text = format!(
        " rstype by Mark Veltzer <mark.veltzer@gmail.com>  [mode: {mode_label}]{}",
        " ".repeat(area.width as usize)
    );
    frame.render_widget(Paragraph::new(text).style(style), area);
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

    let title = match app.typing_state {
        TypingState::Waiting => " press any key to start ",
        _ => " typing… ",
    };

    let text_width = (app.target.len() as u16 + 4).min(area.width);
    let box_rect = centered_rect(text_width, 5, area);

    let paragraph = Paragraph::new(vec![Line::from(""), Line::from(spans)])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
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
        "[{}{}] {:.0}%",
        "█".repeat(filled),
        "░".repeat(empty),
        progress_pct * 100.0,
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

    // Live stats row (only while typing)
    if app.typing_state == TypingState::Typing {
        let live_wpm = if let Some(start) = app.start_time {
            let mins = start.elapsed().as_secs_f64() / 60.0;
            if mins > 0.0 { (app.cursor as f64 / 5.0 / mins) as u32 } else { 0 }
        } else { 0 };
        let total_keys = app.cursor + app.errors;
        let accuracy = if total_keys > 0 {
            (app.cursor.saturating_sub(app.errors) as f64 / total_keys as f64 * 100.0) as u32
        } else { 100 };
        let stats_text = format!(
            "WPM: {}   accuracy: {}%   errors: {}",
            live_wpm, accuracy, app.errors
        );
        let stats_rect = Rect::new(box_rect.x, bar_rect.y + 1, box_rect.width, 1);
        if stats_rect.bottom() <= area.bottom() {
            frame.render_widget(
                Paragraph::new(stats_text)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Cyan)),
                stats_rect,
            );
        }
    }
}

fn render_calendar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let year = app.calendar_year;
    let month = app.calendar_month;
    let first_dow = first_weekday_of_month(year, month); // 0=Mon..6=Sun
    let num_days = days_in_month_cal(year, month);

    const CELL: usize = 10;
    let dow_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    let header_spans: Vec<Span> = dow_names
        .iter()
        .map(|n| Span::styled(
            format!("{:<CELL$}", n),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))
        .collect();

    let mut lines: Vec<Line> = vec![Line::from(""), Line::from(header_spans), Line::from("")];

    // Always render 6 weeks so the box size never changes between months
    let total_slots = 6 * 7;
    let mut day_spans: Vec<Span> = Vec::new();
    let mut stat_spans: Vec<Span> = Vec::new();

    for slot in 0..total_slots {
        let col = slot % 7;
        let day_num = slot as i32 - first_dow as i32 + 1;

        if day_num < 1 || day_num > num_days as i32 {
            day_spans.push(Span::raw(format!("{:<CELL$}", "")));
            stat_spans.push(Span::raw(format!("{:<CELL$}", "")));
        } else {
            let d = day_num as u32;
            let date_key = format!("{:04}-{:02}-{:02}", year, month, d);
            let stat_text = if let Some(&(count, avg)) = app.calendar_stats.get(&date_key) {
                format!("{}s {:.0}w", count, avg)
            } else {
                String::new()
            };
            day_spans.push(Span::styled(
                format!("{:<CELL$}", d),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ));
            stat_spans.push(Span::styled(
                format!("{:<CELL$}", stat_text),
                Style::default().fg(Color::Cyan),
            ));
        }

        // End of week — flush
        if col == 6 {
            lines.push(Line::from(day_spans.clone()));
            lines.push(Line::from(stat_spans.clone()));
            lines.push(Line::from(""));
            day_spans.clear();
            stat_spans.clear();
        }
    }

    let title = format!(" {} {}   ← prev   → next ", month_name(month), year);
    let box_width  = (CELL * 7 + 4) as u16;
    let box_height = (2 + 1 + 6 * 3 + 2) as u16; // header + blank + 6*(day+stat+blank) + borders
    let box_rect = centered_rect(box_width, box_height, area);

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_alignment(Alignment::Center)
                .style(Style::default().fg(Color::Cyan)),
        ),
        box_rect,
    );
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
    let config = load_config();

    // Check terminal size before entering raw/alternate mode
    let (cols, rows) = crossterm::terminal::size()?;
    if cols < config.min_cols || rows < config.min_rows {
        eprintln!(
            "Error: terminal too small (current: {}×{}, required: {}×{})",
            cols, rows, config.min_cols, config.min_rows
        );
        std::process::exit(1);
    }

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

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
