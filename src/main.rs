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
use ratatui::widgets::{Block, Paragraph};
use ratatui::Terminal;
use serde::{Deserialize, Serialize};

const TARGET_TEXT: &str = "\
To be, or not to be, that is the question: \
Whether 'tis nobler in the mind to suffer \
The slings and arrows of outrageous fortune, \
Or to take arms against a sea of troubles \
And by opposing end them. To die—to sleep, \
No more; and by a sleep to say we end \
The heart-ache and the thousand natural shocks \
That flesh is heir to: 'tis a consummation \
Devoutly to be wish'd. To die, to sleep; \
To sleep, perchance to dream—ay, there's the rub: \
For in that sleep of death what dreams may come, \
When we have shuffled off this mortal coil, \
Must give us pause—there's the respect \
That makes calamity of so long life. \
For who would bear the whips and scorns of time, \
The oppressor's wrong, the proud man's contumely, \
The pangs of dispriz'd love, the law's delay, \
The insolence of office, and the spurns \
That patient merit of the unworthy takes, \
When he himself might his quietus make \
With a bare bodkin?";

// ── Text fetching ─────────────────────────────────────────────────────────────

fn fetch_text(source: TextSource) -> String {
    match source {
        TextSource::Wikipedia => fetch_wikipedia().unwrap_or_else(|| TARGET_TEXT.to_string()),
        TextSource::WordSalad => TARGET_TEXT.to_string(), // placeholder
    }
}

fn fetch_wikipedia() -> Option<String> {
    let resp = ureq::get("https://en.wikipedia.org/api/rest_v1/page/random/summary")
        .set("User-Agent", "rstype/1.0 (typing trainer)")
        .call()
        .ok()?;
    let json: serde_json::Value = resp.into_json().ok()?;
    let extract = json.get("extract")?.as_str()?;
    // Take the first meaningful paragraph, capped at ~600 chars
    let para = extract
        .split('\n')
        .find(|p| p.len() > 80)
        .unwrap_or(extract);
    let trimmed: String = para.chars().take(600).collect();
    // Snap back to last sentence boundary if we cut mid-sentence
    if let Some(pos) = trimmed.rfind(|c| c == '.' || c == '?' || c == '!') {
        Some(trimmed[..=pos].trim().to_string())
    } else {
        Some(trimmed.trim().to_string())
    }
}



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
    if cfg!(test) { return; }
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
// DayStats: (session_count, avg_wpm, total_words, total_chars)
fn load_history_stats() -> HashMap<String, (usize, f64, usize, usize)> {
    let mut map: HashMap<String, (usize, f64, usize, usize)> = HashMap::new();
    let path = history_path();
    let Ok(content) = fs::read_to_string(&path) else { return map; };
    for line in content.lines() {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if let (Some(ts), Some(wpm)) = (
                val.get("timestamp").and_then(|v| v.as_str()),
                val.get("wpm").and_then(|v| v.as_f64()),
            ) {
                let date_key = ts.get(..10).unwrap_or("").to_string();
                if date_key.len() == 10 {
                    let text = val.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let chars = text.chars().count();
                    let words = text.split_whitespace().count();
                    let entry = map.entry(date_key).or_insert((0, 0.0, 0, 0));
                    entry.0 += 1;
                    entry.1 += wpm;
                    entry.2 += words;
                    entry.3 += chars;
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
    /// Like Forward but must correct all errors (backspace works); only finishes when fully correct.
    Correct,
    /// One mistake resets the entire session immediately.
    SuddenDeath,
    /// Typed characters are hidden (shown as ·); no visual feedback.
    Blind,
}

impl TypingMode {
    fn label(self) -> &'static str {
        match self {
            TypingMode::Forward     => "Forward",
            TypingMode::Stop        => "Stop",
            TypingMode::Correct     => "Correct",
            TypingMode::SuddenDeath => "Sudden Death",
            TypingMode::Blind       => "Blind",
        }
    }

    fn description(self) -> &'static str {
        match self {
            TypingMode::Forward     => "Wrong key advances the cursor. The mistake is marked in red.\nSpeed matters more than accuracy.",
            TypingMode::Stop        => "Wrong key is blocked — cursor stays put until you press\nthe right key. No mistakes recorded.",
            TypingMode::Correct     => "Wrong key advances but is marked in red. Backspace works.\nYou cannot finish until every character is correct.",
            TypingMode::SuddenDeath => "One wrong key resets the session immediately.\nPerfect accuracy is required to finish.",
            TypingMode::Blind       => "Typed characters are hidden as ·. No red/green feedback.\nTrust your muscle memory.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TextSource {
    Wikipedia,
    WordSalad,
}

impl TextSource {
    fn label(self) -> &'static str {
        match self {
            TextSource::Wikipedia => "Wikipedia",
            TextSource::WordSalad => "Word Salad",
        }
    }

    fn description(self) -> &'static str {
        match self {
            TextSource::Wikipedia => "Fetch a random Wikipedia article summary.\nRequires an internet connection.",
            TextSource::WordSalad => "Generate a random sequence of common English words.\nNo internet required. (not yet implemented)",
        }
    }
}

fn default_text_source() -> TextSource { TextSource::Wikipedia }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    mode: TypingMode,
    #[serde(default = "default_text_source")]
    text_source: TextSource,
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
            text_source: default_text_source(),
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

#[derive(PartialEq, Clone, Debug)]
enum Screen {
    Typing,
    Config,
    Calendar,
    About,
    Exit,
}

struct App {
    // config (persisted)
    config: Config,

    // typing session
    target: Vec<char>,
    /// Set while background text fetch is in progress.
    fetching: bool,
    fetch_rx: Option<std::sync::mpsc::Receiver<String>>,
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
    /// Which config section is focused: 0 = typing mode, 1 = text source.
    config_section: usize,
    /// Selected index within the mode list.
    config_cursor: usize,
    /// Selected index within the text source list.
    config_source_cursor: usize,
    // calendar
    calendar_year: i32,
    calendar_month: u32,
    calendar_stats: HashMap<String, (usize, f64, usize, usize)>,
}

#[derive(PartialEq, Debug)]
enum TypingState {
    Waiting,
    Typing,
    Done,
}

impl App {
    fn new(config: Config) -> Self {
        let (fetching, fetch_rx, target) = if cfg!(test) {
            // Tests don't fetch; use the built-in text synchronously.
            (false, None, TARGET_TEXT.chars().collect())
        } else {
            let source = config.text_source;
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let text = fetch_text(source);
                let _ = tx.send(text);
            });
            (true, Some(rx), Vec::new())
        };
        Self {
            config,
            target,
            fetching,
            fetch_rx,
            typed: Vec::new(),
            cursor: 0,
            errors: 0,
            error_flash: false,
            typing_state: TypingState::Waiting,
            start_time: None,
            wpm: 0.0,
            keystrokes: Vec::new(),
            screen: Screen::Typing,
            config_section: 0,
            config_cursor: 0,
            config_source_cursor: 0,
            calendar_year: 0,
            calendar_month: 1,
            calendar_stats: HashMap::new(),
        }
    }

    /// Poll the background fetch channel; call each frame.
    fn poll_fetch(&mut self) {
        if let Some(rx) = &self.fetch_rx {
            if let Ok(text) = rx.try_recv() {
                self.target = text.chars().collect();
                self.fetching = false;
                self.fetch_rx = None;
            }
        }
    }

    fn restart(&mut self) {
        let cfg = self.config.clone();
        *self = App::new(cfg);
    }

    /// Returns true if the app should quit.
    fn on_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // Always allow Ctrl+C to quit
        if ctrl && key.code == KeyCode::Char('c') { return true; }

        // Block all other input while fetching
        if self.fetching { return false; }

        // Global Ctrl shortcuts — work from any screen
        if ctrl {
            match key.code {
                KeyCode::Char('c') => return true, // standard kill, always works
                KeyCode::Char('e') => {
                    self.screen = Screen::Exit;
                    return false;
                }
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
                KeyCode::Char('a') => {
                    self.screen = Screen::About;
                    return false;
                }
                _ => {}
            }
        }

        // Left/Right arrows cycle through toolbar screens on all screens,
        // but not while a typing session is in progress
        let typing_in_progress = self.screen == Screen::Typing
            && self.typing_state == TypingState::Typing;
        if !typing_in_progress {
            const ORDER: [Screen; 5] = [Screen::Typing, Screen::Config, Screen::Calendar, Screen::About, Screen::Exit];
            let cur = ORDER.iter().position(|s| s == &self.screen).unwrap_or(0);
            if key.code == KeyCode::Left {
                self.screen = ORDER[(cur + ORDER.len() - 1) % ORDER.len()].clone();
                if self.screen == Screen::Calendar { self.open_calendar(); }
                if self.screen == Screen::Config   { self.open_config(); }
                return false;
            }
            if key.code == KeyCode::Right {
                self.screen = ORDER[(cur + 1) % ORDER.len()].clone();
                if self.screen == Screen::Calendar { self.open_calendar(); }
                if self.screen == Screen::Config   { self.open_config(); }
                return false;
            }
        }

        // Esc goes back to train screen (from any non-typing screen), or quits if on train
        if key.code == KeyCode::Esc {
            match self.screen {
                Screen::Config | Screen::Calendar | Screen::About => {
                    self.screen = Screen::Typing;
                    return false;
                }
                Screen::Exit => return true,
                Screen::Typing => return true,
            }
        }

        // On Exit screen: Enter confirms quit
        if self.screen == Screen::Exit && key.code == KeyCode::Enter {
            return true;
        }

        match self.screen {
            Screen::Config   => self.on_key_config(key),
            Screen::Typing   => self.on_key_typing(key),
            Screen::Calendar => self.on_key_calendar(key),
            Screen::About    => {}
            Screen::Exit     => {}
        }
        false
    }

    fn on_key_config(&mut self, key: KeyEvent) {
        const MODES: [TypingMode; 5] = [TypingMode::Forward, TypingMode::Stop, TypingMode::Correct, TypingMode::SuddenDeath, TypingMode::Blind];
        const SOURCES: [TextSource; 2] = [TextSource::Wikipedia, TextSource::WordSalad];
        match key.code {
            KeyCode::Tab => {
                // Switch between mode section and source section
                self.config_section = (self.config_section + 1) % 2;
            }
            KeyCode::Up => {
                if self.config_section == 0 {
                    if self.config_cursor > 0 { self.config_cursor -= 1; }
                } else {
                    if self.config_source_cursor > 0 { self.config_source_cursor -= 1; }
                }
            }
            KeyCode::Down => {
                if self.config_section == 0 {
                    if self.config_cursor + 1 < MODES.len() { self.config_cursor += 1; }
                } else {
                    if self.config_source_cursor + 1 < SOURCES.len() { self.config_source_cursor += 1; }
                }
            }
            KeyCode::Enter => {
                self.config.mode = MODES[self.config_cursor];
                self.config.text_source = SOURCES[self.config_source_cursor];
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
                            TypingMode::Forward | TypingMode::Correct | TypingMode::Blind => {
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
                            TypingMode::SuddenDeath => {
                                if ch == expected {
                                    self.typed.push(ch);
                                    self.cursor += 1;
                                } else {
                                    // Reset immediately
                                    self.restart();
                                    return;
                                }
                            }
                        }

                        // In Correct mode, only finish when every character matches
                        let all_correct = self.typed.iter().zip(self.target.iter()).all(|(a, b)| a == b);
                        let at_end = self.cursor == self.target.len();
                        if at_end && (self.config.mode != TypingMode::Correct || all_correct) {
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
        self.config_section = 0;
        self.config_cursor = match self.config.mode {
            TypingMode::Forward     => 0,
            TypingMode::Stop        => 1,
            TypingMode::Correct     => 2,
            TypingMode::SuddenDeath => 3,
            TypingMode::Blind       => 4,
        };
        self.config_source_cursor = match self.config.text_source {
            TextSource::Wikipedia => 0,
            TextSource::WordSalad => 1,
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
            KeyCode::Char(',') => {
                if self.calendar_month == 1 {
                    self.calendar_month = 12;
                    self.calendar_year -= 1;
                } else {
                    self.calendar_month -= 1;
                }
            }
            KeyCode::Char('.') => {
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

        // Reserve top row for toolbar, bottom row for status bar, one blank line below toolbar
        let toolbar_rect   = Rect::new(area.x, area.y, area.width, 1);
        let statusbar_rect = Rect::new(area.x, area.y + area.height.saturating_sub(1), area.width, 1);
        let body_rect      = Rect::new(area.x, area.y + 2, area.width, area.height.saturating_sub(3));

        render_toolbar(frame, toolbar_rect, app);
        render_statusbar(frame, statusbar_rect, app);

        // 2-column left indent for content screens
        let indent = 2u16;
        let indented = Rect::new(
            body_rect.x + indent,
            body_rect.y,
            body_rect.width.saturating_sub(indent),
            body_rect.height,
        );

        match app.screen {
            Screen::Config   => render_config(frame, body_rect, app),
            Screen::Calendar => render_calendar(frame, indented, app),
            Screen::About    => render_about(frame, indented),
            Screen::Exit     => render_exit(frame, indented),
            Screen::Typing   => match app.typing_state {
                TypingState::Done => render_done(frame, body_rect, app),
                _ => render_typing(frame, indented, app),
            },
        }
    })?;
    Ok(())
}

fn render_toolbar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let active_style     = Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD);
    let normal_style     = Style::default().fg(Color::Black).bg(Color::White);
    let key_style        = Style::default().fg(Color::Blue).bg(Color::White).add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let active_key_style = Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let sep_style        = Style::default().fg(Color::DarkGray).bg(Color::White);

    let is_train    = app.screen == Screen::Typing;
    let is_config   = app.screen == Screen::Config;
    let is_calendar = app.screen == Screen::Calendar;
    let is_about    = app.screen == Screen::About;
    let is_exit     = app.screen == Screen::Exit;

    let mut spans = vec![Span::styled("  ", normal_style)];

    for (before, key, after, active) in [
        ("",      'T', "rain",   is_train),
        ("Confi", 'G', "",       is_config),
        ("",      'H', "istory", is_calendar),
        ("",      'A', "bout",   is_about),
        ("",      'E', "xit",    is_exit),
    ] {
        let (ks, rs) = if active { (active_key_style, active_style) } else { (key_style, normal_style) };
        if !before.is_empty() {
            spans.push(Span::styled(before, rs));
        }
        spans.push(Span::styled(key.to_string(), ks));
        if !after.is_empty() {
            spans.push(Span::styled(after, rs));
        }
        spans.push(Span::styled("  ", sep_style));
    }

    // Right-align "rstype by Mark Veltzer" in the remaining space
    // Left side: "  " + "Train  " + "ConfiG  " + "History  " + "About  " + "Exit  " = 2+8+8+9+8+6 = 41
    let left_width: u16 = 41;
    let title = "rstype by Mark Veltzer  ";
    let title_len = title.len() as u16;
    let pad = area.width.saturating_sub(left_width + title_len);
    spans.push(Span::styled(" ".repeat(pad as usize), normal_style));
    spans.push(Span::styled(title, Style::default().fg(Color::DarkGray).bg(Color::White)));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_statusbar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let style = Style::default().fg(Color::Black).bg(Color::White);
    let mode_label = match app.config.mode {
        TypingMode::Forward      => "forward",
        TypingMode::Stop         => "stop",
        TypingMode::Correct      => "correct",
        TypingMode::SuddenDeath  => "sudden death",
        TypingMode::Blind        => "blind",
    };
    let text = format!(" mode: {mode_label}{}", " ".repeat(area.width as usize));
    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn render_typing(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    if app.fetching {
        let msg = Line::from(Span::styled(
            "  Fetching text from Wikipedia…",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(vec![Line::from(""), msg]), area);
        return;
    }

    let blind = app.config.mode == TypingMode::Blind;

    // ── Word-wrap the target into lines that fit the display width ─────────
    let max_w = (area.width as usize).saturating_sub(4).max(20);
    // Build a list of (start_offset, end_offset) char ranges per visual line
    let mut lines_ranges: Vec<(usize, usize)> = Vec::new();
    let text = &app.target;
    let mut pos = 0;
    while pos < text.len() {
        // Find how many chars fit: try to break at a space
        let remaining = text.len() - pos;
        let take = remaining.min(max_w);
        let end = if pos + take >= text.len() {
            text.len()
        } else {
            // Look back for a space to break on
            let slice = &text[pos..pos + take];
            if let Some(sp) = slice.iter().rposition(|&c| c == ' ') {
                pos + sp + 1 // break after the space
            } else {
                pos + take // no space, hard break
            }
        };
        lines_ranges.push((pos, end));
        pos = end;
    }

    // Find which visual line the cursor is on
    let cursor_line = lines_ranges
        .iter()
        .position(|&(s, e)| app.cursor >= s && app.cursor < e.max(s + 1))
        .unwrap_or(lines_ranges.len().saturating_sub(1));

    // Reserve rows: progress bar + stats + some margin
    let reserved = 5u16;
    let viewport_h = area.height.saturating_sub(reserved) as usize;
    let viewport_h = viewport_h.max(1);

    // Scroll so cursor line stays in the middle of the viewport
    let scroll_top = if cursor_line < viewport_h / 2 {
        0
    } else {
        cursor_line - viewport_h / 2
    };

    let view_y = area.y; // start from top
    for (row, &(start, end)) in lines_ranges
        .iter()
        .enumerate()
        .skip(scroll_top)
        .take(viewport_h)
    {
        let mut spans: Vec<Span> = Vec::new();
        for i in start..end {
            let ch = text[i];
            let span = if i < app.cursor {
                if blind {
                    Span::styled("·", Style::default().fg(Color::DarkGray))
                } else if app.typed[i] == ch {
                    Span::styled(ch.to_string(), Style::default().fg(Color::Green))
                } else {
                    Span::styled(ch.to_string(), Style::default().fg(Color::Red).add_modifier(Modifier::UNDERLINED))
                }
            } else if i == app.cursor {
                let bg = if app.error_flash { Color::Red } else { Color::White };
                Span::styled(ch.to_string(), Style::default().fg(Color::Black).bg(bg).add_modifier(Modifier::BOLD))
            } else {
                Span::styled(ch.to_string(), Style::default().fg(Color::DarkGray))
            };
            spans.push(span);
        }
        let render_row = view_y + (row - scroll_top) as u16;
        if render_row < area.bottom().saturating_sub(reserved) {
            let line_rect = Rect::new(area.x, render_row, area.width, 1);
            frame.render_widget(Paragraph::new(Line::from(spans)), line_rect);
        }
    }

    // ── Progress bar ──────────────────────────────────────────────────────
    let progress_pct = app.cursor as f64 / app.target.len() as f64;
    let bar_w = area.width as usize;
    let filled = (bar_w as f64 * progress_pct) as usize;
    let empty = bar_w - filled;
    let bar_text = format!("[{}{}] {:.0}%", "█".repeat(filled), "░".repeat(empty), progress_pct * 100.0);
    let bar_y = area.bottom().saturating_sub(reserved - 1);
    let bar_rect = Rect::new(area.x, bar_y, area.width, 1);
    frame.render_widget(
        Paragraph::new(bar_text).style(Style::default().fg(Color::Yellow)),
        bar_rect,
    );

    // ── Live stats ────────────────────────────────────────────────────────
    if app.typing_state == TypingState::Typing {
        let live_wpm = if let Some(start) = app.start_time {
            let mins = start.elapsed().as_secs_f64() / 60.0;
            if mins > 0.0 { (app.cursor as f64 / 5.0 / mins) as u32 } else { 0 }
        } else { 0 };
        let total_keys = app.cursor + app.errors;
        let accuracy = if total_keys > 0 {
            (app.cursor.saturating_sub(app.errors) as f64 / total_keys as f64 * 100.0) as u32
        } else { 100 };
        let elapsed = app.start_time.map(|s| s.elapsed().as_secs()).unwrap_or(0);
        let stats_text = format!(
            "WPM: {}   accuracy: {}%   errors: {}   time: {}:{:02}",
            live_wpm, accuracy, app.errors, elapsed / 60, elapsed % 60
        );
        let stats_rect = Rect::new(area.x, bar_y + 2, area.width, 1);
        if stats_rect.bottom() <= area.bottom() {
            frame.render_widget(
                Paragraph::new(stats_text).style(Style::default().fg(Color::Cyan)),
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
    let mut wc_spans: Vec<Span> = Vec::new();

    for slot in 0..total_slots {
        let col = slot % 7;
        let day_num = slot as i32 - first_dow as i32 + 1;

        if day_num < 1 || day_num > num_days as i32 {
            day_spans.push(Span::raw(format!("{:<CELL$}", "")));
            stat_spans.push(Span::raw(format!("{:<CELL$}", "")));
            wc_spans.push(Span::raw(format!("{:<CELL$}", "")));
        } else {
            let d = day_num as u32;
            let date_key = format!("{:04}-{:02}-{:02}", year, month, d);
            let (stat_text, wc_text) = if let Some(&(count, avg, words, chars)) = app.calendar_stats.get(&date_key) {
                (format!("{}s {:.0}wpm", count, avg), format!("{}w {}c", words, chars))
            } else {
                (String::new(), String::new())
            };
            day_spans.push(Span::styled(
                format!("{:<CELL$}", d),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ));
            stat_spans.push(Span::styled(
                format!("{:<CELL$}", stat_text),
                Style::default().fg(Color::Cyan),
            ));
            wc_spans.push(Span::styled(
                format!("{:<CELL$}", wc_text),
                Style::default().fg(Color::Yellow),
            ));
        }

        // End of week — flush
        if col == 6 {
            lines.push(Line::from(day_spans.clone()));
            lines.push(Line::from(stat_spans.clone()));
            lines.push(Line::from(wc_spans.clone()));
            lines.push(Line::from(""));
            day_spans.clear();
            stat_spans.clear();
            wc_spans.clear();
        }
    }

    let title = format!("  {} {}   , prev   . next", month_name(month), year);
    let mut all_lines = vec![
        Line::from(""),
        Line::from(Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
    ];
    all_lines.extend(lines);

    frame.render_widget(Paragraph::new(all_lines), area);
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
        Line::from(Span::styled(
            "── Results ──",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("Speed:    "),
            Span::styled(
                format!("{:.1} WPM", app.wpm),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Accuracy: "),
            Span::styled(
                format!("{:.1}%", accuracy),
                Style::default().fg(acc_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Errors:   "),
            Span::styled(
                format!("{}", app.errors),
                Style::default()
                    .fg(if app.errors == 0 { Color::Green } else { Color::Red })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Enter / Space / R to restart",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let result_rect = centered_rect(36, lines.len() as u16, area);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        result_rect,
    );
}

fn render_exit(frame: &mut ratatui::Frame, area: Rect) {
    // Load today's stats fresh
    let stats = load_history_stats();
    let (year, month) = today_ym();
    // today_ym gives year/month; we need today's full date key
    // Compute today's day from system time
    let today_day = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let (_, _, d) = days_to_ymd(secs / 86400);
        d as u32
    };
    let date_key = format!("{:04}-{:02}-{:02}", year, month, today_day);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Today's training",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if let Some(&(sessions, avg_wpm, words, chars)) = stats.get(&date_key) {
        let row = |label: &str, value: String| -> Line<'static> {
            Line::from(vec![
                Span::styled(format!("  {:<16}", label), Style::default().fg(Color::DarkGray)),
                Span::styled(value, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ])
        };
        lines.push(row("sessions", sessions.to_string()));
        lines.push(row("avg WPM", format!("{:.1}", avg_wpm)));
        lines.push(row("total words", words.to_string()));
        lines.push(row("total chars", chars.to_string()));
    } else {
        lines.push(Line::from(Span::styled(
            "  No sessions today yet.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press Enter or Esc to quit",
        Style::default().fg(Color::DarkGray),
    )));

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_about(frame: &mut ratatui::Frame, area: Rect) {
    let w = |label: &str, value: &str, color: Color| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:<18}", label), Style::default().fg(Color::DarkGray)),
            Span::styled(value.to_string(), Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ])
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  rstype — terminal typing trainer",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  by Mark Veltzer <mark.veltzer@gmail.com>",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        w("version",         env!("CARGO_PKG_VERSION"),  Color::Yellow),
        w("git describe",    env!("GIT_DESCRIBE"),        Color::Yellow),
        w("git branch",      env!("GIT_BRANCH"),          Color::Yellow),
        w("git sha",         env!("GIT_SHA"),              Color::Yellow),
        w("dirty",           env!("GIT_DIRTY"),            Color::Yellow),
        w("built",           env!("BUILD_TIMESTAMP"),      Color::Yellow),
        w("rustc",           env!("RUSTC_SEMVER"),         Color::Yellow),
        w("rust edition",    env!("RUST_EDITION"),         Color::Yellow),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_config(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    const MODES: [TypingMode; 5] = [TypingMode::Forward, TypingMode::Stop, TypingMode::Correct, TypingMode::SuddenDeath, TypingMode::Blind];
    const SOURCES: [TextSource; 2] = [TextSource::Wikipedia, TextSource::WordSalad];

    let section_style = |active: bool| -> Style {
        if active {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Typing mode  (Tab to switch section, ↑↓ to move, Enter to save)",
            section_style(app.config_section == 0),
        )),
        Line::from(""),
    ];

    for (i, mode) in MODES.iter().enumerate() {
        let selected = app.config_section == 0 && i == app.config_cursor;
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
    }

    if app.config_section == 0 {
        lines.push(Line::from(""));
        let desc = MODES[app.config_cursor].description();
        for desc_line in desc.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", desc_line),
                Style::default().fg(Color::White),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Text source",
        section_style(app.config_section == 1),
    )));
    lines.push(Line::from(""));

    for (i, src) in SOURCES.iter().enumerate() {
        let selected = app.config_section == 1 && i == app.config_source_cursor;
        let active = *src == app.config.text_source;
        let prefix = if selected { "▶ " } else { "  " };
        let suffix = if active { "  ✓" } else { "" };
        let label = format!("{}{}{}", prefix, src.label(), suffix);
        let style = if selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(Span::styled(label, style)));
    }

    if app.config_section == 1 {
        lines.push(Line::from(""));
        let desc = SOURCES[app.config_source_cursor].description();
        for desc_line in desc.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", desc_line),
                Style::default().fg(Color::White),
            )));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
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
        app.poll_fetch();
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    // ── Calendar helpers ──────────────────────────────────────────────────

    #[test]
    fn leap_year_divisible_by_4() {
        assert!(is_leap_year(2024));
    }

    #[test]
    fn leap_year_century_not_leap() {
        assert!(!is_leap_year(1900));
    }

    #[test]
    fn leap_year_400_is_leap() {
        assert!(is_leap_year(2000));
    }

    #[test]
    fn days_in_february_leap() {
        assert_eq!(days_in_month_cal(2024, 2), 29);
    }

    #[test]
    fn days_in_february_non_leap() {
        assert_eq!(days_in_month_cal(2023, 2), 28);
    }

    #[test]
    fn days_in_month_30_and_31() {
        assert_eq!(days_in_month_cal(2024, 4), 30);  // April
        assert_eq!(days_in_month_cal(2024, 1), 31);  // January
        assert_eq!(days_in_month_cal(2024, 12), 31); // December
    }

    #[test]
    fn first_weekday_known_date() {
        // 2024-01-01 was a Monday (0)
        assert_eq!(first_weekday_of_month(2024, 1), 0);
        // 2024-04-01 was a Monday (0)
        assert_eq!(first_weekday_of_month(2024, 4), 0);
        // 2024-03-01 was a Friday (4)
        assert_eq!(first_weekday_of_month(2024, 3), 4);
    }

    #[test]
    fn days_to_ymd_epoch() {
        // Day 0 = 1970-01-01
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_known() {
        // 2024-01-01: days since epoch = 19723
        assert_eq!(days_to_ymd(19723), (2024, 1, 1));
    }

    // ── W3C key names ─────────────────────────────────────────────────────

    #[test]
    fn w3c_regular_char() {
        assert_eq!(keycode_to_w3c(KeyCode::Char('a')), "a");
    }

    #[test]
    fn w3c_space() {
        assert_eq!(keycode_to_w3c(KeyCode::Char(' ')), "Space");
    }

    #[test]
    fn w3c_backspace() {
        assert_eq!(keycode_to_w3c(KeyCode::Backspace), "Backspace");
    }

    #[test]
    fn w3c_enter() {
        assert_eq!(keycode_to_w3c(KeyCode::Enter), "Enter");
    }

    // ── TypingMode ────────────────────────────────────────────────────────

    #[test]
    fn mode_labels_distinct() {
        let labels: Vec<_> = [
            TypingMode::Forward, TypingMode::Stop, TypingMode::Correct,
            TypingMode::SuddenDeath, TypingMode::Blind,
        ].iter().map(|m| m.label()).collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len());
    }

    #[test]
    fn mode_descriptions_non_empty() {
        for m in [TypingMode::Forward, TypingMode::Stop, TypingMode::Correct,
                  TypingMode::SuddenDeath, TypingMode::Blind] {
            assert!(!m.description().is_empty());
        }
    }

    // ── App typing logic ──────────────────────────────────────────────────

    fn app_with_text(text: &str, mode: TypingMode) -> App {
        let mut config = Config::default();
        config.mode = mode;
        let mut app = App::new(config);
        app.target = text.chars().collect();
        app.typed = Vec::new();
        app.cursor = 0;
        app.errors = 0;
        app
    }

    #[test]
    fn forward_mode_correct_key_advances() {
        let mut app = app_with_text("ab", TypingMode::Forward);
        app.on_key(key(KeyCode::Char('a')));
        assert_eq!(app.cursor, 1);
        assert_eq!(app.errors, 0);
    }

    #[test]
    fn forward_mode_wrong_key_still_advances() {
        let mut app = app_with_text("ab", TypingMode::Forward);
        app.on_key(key(KeyCode::Char('x')));
        assert_eq!(app.cursor, 1);
        assert_eq!(app.errors, 1);
    }

    #[test]
    fn stop_mode_wrong_key_does_not_advance() {
        let mut app = app_with_text("ab", TypingMode::Stop);
        app.on_key(key(KeyCode::Char('x')));
        assert_eq!(app.cursor, 0);
        assert_eq!(app.errors, 1);
    }

    #[test]
    fn stop_mode_correct_key_advances() {
        let mut app = app_with_text("ab", TypingMode::Stop);
        app.on_key(key(KeyCode::Char('a')));
        assert_eq!(app.cursor, 1);
        assert_eq!(app.errors, 0);
    }

    #[test]
    fn correct_mode_cannot_finish_with_errors() {
        let mut app = app_with_text("ab", TypingMode::Correct);
        app.on_key(key(KeyCode::Char('x'))); // wrong
        app.on_key(key(KeyCode::Char('b'))); // reaches end but first char wrong
        assert_ne!(app.typing_state, TypingState::Done);
    }

    #[test]
    fn correct_mode_finishes_when_all_correct() {
        let mut app = app_with_text("ab", TypingMode::Correct);
        app.on_key(key(KeyCode::Char('x'))); // wrong
        app.on_key(key(KeyCode::Backspace));  // go back
        app.on_key(key(KeyCode::Char('a'))); // correct
        app.on_key(key(KeyCode::Char('b'))); // correct
        assert_eq!(app.typing_state, TypingState::Done);
    }

    #[test]
    fn sudden_death_resets_on_wrong_key() {
        let mut app = app_with_text("abc", TypingMode::SuddenDeath);
        app.on_key(key(KeyCode::Char('a'))); // correct
        app.on_key(key(KeyCode::Char('x'))); // wrong — should reset
        assert_eq!(app.cursor, 0);
        assert_eq!(app.typing_state, TypingState::Waiting);
    }

    #[test]
    fn backspace_moves_cursor_back() {
        let mut app = app_with_text("abc", TypingMode::Forward);
        app.on_key(key(KeyCode::Char('a')));
        app.on_key(key(KeyCode::Char('b')));
        app.on_key(key(KeyCode::Backspace));
        assert_eq!(app.cursor, 1);
    }

    #[test]
    fn ctrl_c_quits() {
        let mut app = app_with_text("abc", TypingMode::Forward);
        assert!(app.on_key(ctrl(KeyCode::Char('c'))));
    }

    #[test]
    fn ctrl_t_goes_to_typing_screen() {
        let mut app = app_with_text("abc", TypingMode::Forward);
        app.screen = Screen::Config;
        app.on_key(ctrl(KeyCode::Char('t')));
        assert_eq!(app.screen, Screen::Typing);
    }

    #[test]
    fn forward_mode_completes_on_last_char() {
        let mut app = app_with_text("a", TypingMode::Forward);
        app.on_key(key(KeyCode::Char('a')));
        assert_eq!(app.typing_state, TypingState::Done);
    }
}
