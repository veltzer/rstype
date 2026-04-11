use std::path::PathBuf;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crossterm::event::KeyCode;

pub const FALLBACK_TEXT: &str = "\
No paragraphs collected yet. Please run: rstype wikipedia download";

pub const TEST_TEXT: &str = "\
The quick brown fox jumps over the lazy dog. \
Pack my box with five dozen liquor jugs.";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Keystroke {
    pub typed: String,
    pub offset_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Session {
    pub timestamp: String,
    pub text: String,
    pub mode: String,
    pub wpm: f64,
    pub errors: usize,
    pub keystrokes: Vec<Keystroke>,
}

pub fn history_path() -> PathBuf {
    let mut p = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    p.push(".local");
    p.push("share");
    p.push("rstype");
    p.push("history.jsonl");
    p
}

pub fn paragraphs_path() -> PathBuf {
    let mut p = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    p.push(".local");
    p.push("share");
    p.push("rstype");
    p.push("wikipedia.jsonl");
    p
}

pub fn now_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day) = days_to_ymd(secs / 86400);
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, h, m, s)
}

pub fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hand {
    Left,
    Right,
}

pub fn hand_for_char(c: char) -> Option<Hand> {
    match c.to_ascii_lowercase() {
        '`' | '1' | '2' | '3' | '4' | '5' => Some(Hand::Left),
        '~' | '!' | '@' | '#' | '$' | '%' => Some(Hand::Left),
        'q' | 'w' | 'e' | 'r' | 't'       => Some(Hand::Left),
        'a' | 's' | 'd' | 'f' | 'g'       => Some(Hand::Left),
        'z' | 'x' | 'c' | 'v' | 'b'       => Some(Hand::Left),
        '6' | '7' | '8' | '9' | '0' | '-' | '=' => Some(Hand::Right),
        '^' | '&' | '*' | '(' | ')' | '_' | '+' => Some(Hand::Right),
        'y' | 'u' | 'i' | 'o' | 'p' | '[' | ']' | '\\' => Some(Hand::Right),
        'h' | 'j' | 'k' | 'l' | ';' | '\'' => Some(Hand::Right),
        'n' | 'm' | ',' | '.' | '/'        => Some(Hand::Right),
        '{' | '}' | '|' | ':' | '"' | '<' | '>' | '?' => Some(Hand::Right),
        _ => None,
    }
}

#[derive(Debug, Default)]
pub struct HandStats {
    pub avg_response_ms: f64,
    pub total_keys: usize,
    pub errors: usize,
}

impl HandStats {
    pub fn error_rate(&self) -> f64 {
        if self.total_keys == 0 { 0.0 } else { self.errors as f64 / self.total_keys as f64 * 100.0 }
    }
}

pub fn compute_hand_stats(target: &[char], typed: &[char], keystrokes: &[Keystroke]) -> (HandStats, HandStats) {
    let mut left = HandStats::default();
    let mut right = HandStats::default();
    let mut left_total_ms: f64 = 0.0;
    let mut right_total_ms: f64 = 0.0;

    let mut char_times: Vec<u64> = Vec::new();
    let mut prev_offset: Option<u64> = None;
    for ks in keystrokes {
        let is_char = ks.typed.len() == 1 || ks.typed == "Space";
        if is_char {
            if let Some(prev) = prev_offset {
                char_times.push(ks.offset_ms.saturating_sub(prev));
            } else {
                char_times.push(0);
            }
        }
        prev_offset = Some(ks.offset_ms);
    }

    let len = target.len().min(typed.len()).min(char_times.len());
    for i in 0..len {
        let expected = target[i];
        let actual = typed[i];
        let hand = match hand_for_char(expected) {
            Some(h) => h,
            None => continue,
        };

        let stats = match hand {
            Hand::Left  => &mut left,
            Hand::Right => &mut right,
        };
        let total_ms = match hand {
            Hand::Left  => &mut left_total_ms,
            Hand::Right => &mut right_total_ms,
        };

        stats.total_keys += 1;
        if actual != expected {
            stats.errors += 1;
        }
        if i > 0 {
            *total_ms += char_times[i] as f64;
        }
    }

    let left_interval_count = if left.total_keys > 1 { left.total_keys - 1 } else { 0 };
    let right_interval_count = if right.total_keys > 1 { right.total_keys - 1 } else { 0 };
    left.avg_response_ms = if left_interval_count > 0 { left_total_ms / left_interval_count as f64 } else { 0.0 };
    right.avg_response_ms = if right_interval_count > 0 { right_total_ms / right_interval_count as f64 } else { 0.0 };

    (left, right)
}

pub fn keycode_to_w3c(code: KeyCode) -> String {
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

pub fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub fn days_in_month_cal(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 31,
    }
}

pub fn first_weekday_of_month(year: i32, month: u32) -> u32 {
    let t = [0u32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let yu = y as u32;
    let dow = (yu + yu / 4 - yu / 100 + yu / 400 + t[(month - 1) as usize] + 1) % 7;
    (dow + 6) % 7
}

pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January", 2 => "February", 3 => "March",    4 => "April",
        5 => "May",     6 => "June",     7 => "July",      8 => "August",
        9 => "September", 10 => "October", 11 => "November", 12 => "December",
        _ => "?",
    }
}

pub fn today_ym() -> (i32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let (y, m, _) = days_to_ymd(days);
    (y as i32, m as u32)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum TypingMode {
    Forward,
    Stop,
    Correct,
    SuddenDeath,
    Blind,
}

impl TypingMode {
    pub fn label(self) -> &'static str {
        match self {
            TypingMode::Forward     => "Forward",
            TypingMode::Stop        => "Stop",
            TypingMode::Correct     => "Correct",
            TypingMode::SuddenDeath => "Sudden Death",
            TypingMode::Blind       => "Blind",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            TypingMode::Forward     => "Wrong key advances the cursor. The mistake is marked in red.\nSpeed matters more than accuracy.",
            TypingMode::Stop        => "Wrong key is blocked — cursor stays put until you press\nthe right key. No mistakes recorded.",
            TypingMode::Correct     => "Wrong key advances but is marked in red. Backspace works.\nYou cannot finish until every character is correct.",
            TypingMode::SuddenDeath => "One wrong key sends you back to the start of the text.\nThe clock keeps running. Perfect accuracy required to finish.",
            TypingMode::Blind       => "Typed characters are hidden as ·. No red/green feedback.\nTrust your muscle memory.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum TextSource {
    Wikipedia,
    WordSalad,
}

impl TextSource {
    pub fn label(self) -> &'static str {
        match self {
            TextSource::Wikipedia => "Wikipedia",
            TextSource::WordSalad => "Word Salad",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            TextSource::Wikipedia => "Fetch a random Wikipedia article summary.\nRequires an internet connection.",
            TextSource::WordSalad => "Generate a random sequence of common English words.\nNo internet required.",
        }
    }
}

pub fn default_text_source() -> TextSource { TextSource::Wikipedia }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TextLength {
    OneLine,
    ShortParagraph,
    Paragraph,
    LongParagraph,
}

impl TextLength {
    pub fn label(self) -> &'static str {
        match self {
            TextLength::OneLine        => "One line",
            TextLength::ShortParagraph => "Short paragraph",
            TextLength::Paragraph      => "Paragraph",
            TextLength::LongParagraph  => "Long paragraph",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            TextLength::OneLine        => "Around 60 characters — a single sentence.",
            TextLength::ShortParagraph => "Around 150 characters — two or three sentences.",
            TextLength::Paragraph      => "Around 300 characters — a full paragraph.",
            TextLength::LongParagraph  => "Around 600 characters — an extended passage.",
        }
    }

    pub fn max_chars(self) -> usize {
        match self {
            TextLength::OneLine        => 70,
            TextLength::ShortParagraph => 160,
            TextLength::Paragraph      => 320,
            TextLength::LongParagraph  => 640,
        }
    }

    pub fn min_chars(self) -> usize {
        match self {
            TextLength::OneLine        => 30,
            TextLength::ShortParagraph => 80,
            TextLength::Paragraph      => 160,
            TextLength::LongParagraph  => 320,
        }
    }
}

pub fn default_text_length() -> TextLength { TextLength::Paragraph }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mode: TypingMode,
    #[serde(default = "default_text_source")]
    pub text_source: TextSource,
    #[serde(default = "default_text_length")]
    pub text_length: TextLength,
    #[serde(default = "default_min_cols")]
    pub min_cols: u16,
    #[serde(default = "default_min_rows")]
    pub min_rows: u16,
}

pub fn default_min_cols() -> u16 { 76 }
pub fn default_min_rows() -> u16 { 32 }

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: TypingMode::Forward,
            text_source: default_text_source(),
            text_length: default_text_length(),
            min_cols: default_min_cols(),
            min_rows: default_min_rows(),
        }
    }
}

pub fn config_path() -> PathBuf {
    let mut p = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    p.push(".config");
    p.push("rstype.toml");
    p
}

pub fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn load_config() -> Config {
    let path = config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &Config) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(s) = toml::to_string(cfg) {
        let _ = fs::write(&path, s);
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum Screen {
    Typing,
    Config,
    Stats,
    Wikipedia,
    Calendar,
    About,
    Exit,
}

pub fn centered_rect(width: u16, height: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    ratatui::layout::Rect::new(x, y, width.min(area.width), height.min(area.height))
}

pub fn save_session(session: &Session) {
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

pub fn load_history_stats() -> HashMap<String, (usize, f64, usize, usize)> {
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
    for (_, v) in map.iter_mut() {
        v.1 /= v.0 as f64;
    }
    map
}

pub fn load_all_wpms() -> Vec<f64> {
    let path = history_path();
    let Ok(content) = fs::read_to_string(&path) else { return Vec::new(); };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|val| val.get("wpm").and_then(|v| v.as_f64()))
        .collect()
}

pub fn reconstruct_typed(keystrokes: &[Keystroke]) -> Vec<char> {
    let mut typed = Vec::new();
    for ks in keystrokes {
        if ks.typed == "Backspace" {
            typed.pop();
        } else if ks.typed == "Space" {
            typed.push(' ');
        } else if ks.typed.chars().count() == 1 {
            typed.push(ks.typed.chars().next().unwrap());
        }
    }
    typed
}

pub fn load_history_hand_stats() -> (HandStats, HandStats) {
    let mut left = HandStats::default();
    let mut right = HandStats::default();
    let mut left_total_ms: f64 = 0.0;
    let mut right_total_ms: f64 = 0.0;
    let mut left_interval_count: usize = 0;
    let mut right_interval_count: usize = 0;

    let path = history_path();
    let Ok(content) = fs::read_to_string(&path) else { return (left, right); };
    for line in content.lines() {
        let Ok(session) = serde_json::from_str::<Session>(line) else { continue; };
        let target: Vec<char> = session.text.chars().collect();
        let typed = reconstruct_typed(&session.keystrokes);
        let (sl, sr) = compute_hand_stats(&target, &typed, &session.keystrokes);

        left.total_keys += sl.total_keys;
        left.errors += sl.errors;
        right.total_keys += sr.total_keys;
        right.errors += sr.errors;

        if sl.total_keys > 1 {
            let count = sl.total_keys - 1;
            left_total_ms += sl.avg_response_ms * count as f64;
            left_interval_count += count;
        }
        if sr.total_keys > 1 {
            let count = sr.total_keys - 1;
            right_total_ms += sr.avg_response_ms * count as f64;
            right_interval_count += count;
        }
    }

    left.avg_response_ms = if left_interval_count > 0 { left_total_ms / left_interval_count as f64 } else { 0.0 };
    right.avg_response_ms = if right_interval_count > 0 { right_total_ms / right_interval_count as f64 } else { 0.0 };

    (left, right)
}
