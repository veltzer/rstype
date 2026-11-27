use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{Duration, Instant};

use crate::dict::generate_word_salad;
use crate::utils::{
    Config, FALLBACK_TEXT, Hand, HandStats, Keystroke, Screen, Session, TEST_TEXT, TextLength,
    TextSource, TypingMode, centered_rect, compute_hand_stats, f64_to_u32, f64_to_usize,
    hand_for_char, keycode_to_w3c, load_history_stats, now_timestamp, paragraphs_path, save_config,
    save_session, today_ym, usize_to_f64,
};
use crate::wikipedia::{
    WikiCollectMsg, fetch_wikipedia_paragraphs_batch, load_paragraphs, pick_collected_paragraph,
};

#[derive(PartialEq, Eq, Debug)]
pub enum TypingState {
    Waiting,
    Typing,
    Done,
}

pub struct App {
    pub config: Config,
    pub target: Vec<char>,
    pub fetching: bool,
    pub fetch_rx: Option<std::sync::mpsc::Receiver<String>>,
    pub typed: Vec<char>,
    pub cursor: usize,
    pub errors: usize,
    pub error_flash: bool,
    /// Last pressed key and whether it was the expected one.
    pub last_pressed: Option<(char, bool)>,
    pub typing_state: TypingState,
    pub start_time: Option<Instant>,
    pub wpm: f64,
    pub keystrokes: Vec<Keystroke>,
    pub screen: Screen,
    pub config_section: usize,
    pub config_cursor: usize,
    pub config_source_cursor: usize,
    pub config_length_cursor: usize,
    pub calendar_year: i32,
    pub calendar_month: u32,
    pub calendar_stats: HashMap<String, (usize, f64, usize, usize)>,
    pub wiki_collecting: bool,
    pub wiki_collect_rx: Option<std::sync::mpsc::Receiver<WikiCollectMsg>>,
    pub wiki_collected: usize,
    pub wiki_target: usize,
    pub wiki_requests: u32,
}

impl App {
    pub fn new(config: Config) -> Self {
        let (fetching, fetch_rx, target) = if cfg!(test) {
            (false, None, TEST_TEXT.chars().collect())
        } else {
            let source = config.text_source;
            let length = config.text_length;
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let text = fetch_text(source, length);
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
            last_pressed: None,
            typing_state: TypingState::Waiting,
            start_time: None,
            wpm: 0.0,
            keystrokes: Vec::new(),
            screen: Screen::Typing,
            config_section: 0,
            config_cursor: 0,
            config_source_cursor: 0,
            config_length_cursor: 0,
            calendar_year: 0,
            calendar_month: 1,
            calendar_stats: HashMap::new(),
            wiki_collecting: false,
            wiki_collect_rx: None,
            wiki_collected: 0,
            wiki_target: 0,
            wiki_requests: 0,
        }
    }

    pub fn fetch_new_text(&mut self) {
        let source = self.config.text_source;
        let length = self.config.text_length;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let text = fetch_text(source, length);
            let _ = tx.send(text);
        });
        self.target = Vec::new();
        self.typed = Vec::new();
        self.cursor = 0;
        self.errors = 0;
        self.error_flash = false;
        self.last_pressed = None;
        self.typing_state = TypingState::Waiting;
        self.start_time = None;
        self.wpm = 0.0;
        self.keystrokes = Vec::new();
        self.fetching = true;
        self.fetch_rx = Some(rx);
    }

    pub fn poll_fetch(&mut self) {
        if let Some(rx) = &self.fetch_rx
            && let Ok(text) = rx.try_recv()
        {
            self.target = text.chars().collect();
            self.fetching = false;
            self.fetch_rx = None;
        }
    }

    pub fn poll_wiki_collect(&mut self) {
        let mut done = false;
        if let Some(rx) = &self.wiki_collect_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    WikiCollectMsg::Progress(total, reqs) => {
                        self.wiki_collected = total;
                        self.wiki_requests = reqs;
                    }
                    WikiCollectMsg::Done(total) => {
                        self.wiki_collected = total;
                        self.wiki_collecting = false;
                        done = true;
                    }
                }
            }
        }
        if done {
            self.wiki_collect_rx = None;
        }
    }

    pub fn start_wiki_collect(&mut self, target: usize) {
        if self.wiki_collecting {
            return;
        }
        self.wiki_collecting = true;
        self.wiki_target = target;
        self.wiki_collected = 0;
        self.wiki_requests = 0;

        let (tx, rx) = std::sync::mpsc::channel();
        self.wiki_collect_rx = Some(rx);

        std::thread::spawn(move || {
            let path = paragraphs_path();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let existing = load_paragraphs();
            let mut seen: std::collections::HashSet<String> = existing.into_iter().collect();
            let mut total = seen.len();
            let _ = tx.send(WikiCollectMsg::Progress(total, 0));

            let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
                let _ = tx.send(WikiCollectMsg::Done(total));
                return;
            };

            let mut requests = 0u32;
            while total < target {
                requests += 1;
                let batch = fetch_wikipedia_paragraphs_batch();
                for para in batch {
                    if seen.contains(&para) {
                        continue;
                    }
                    seen.insert(para.clone());
                    if let Ok(json) = serde_json::to_string(&serde_json::json!({ "text": para })) {
                        let _ = writeln!(file, "{json}");
                        total += 1;
                    }
                    if total >= target {
                        break;
                    }
                }
                let _ = tx.send(WikiCollectMsg::Progress(total, requests));
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = tx.send(WikiCollectMsg::Done(total));
        });
    }

    pub fn restart(&mut self) {
        let cfg = self.config.clone();
        *self = Self::new(cfg);
    }

    /// Handle a Ctrl shortcut; returns true when the key was consumed.
    fn on_ctrl_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('e') => self.screen = Screen::Exit,
            KeyCode::Char('t') => self.screen = Screen::Typing,
            KeyCode::Char('g') => self.open_config(),
            KeyCode::Char('h') => self.open_calendar(),
            KeyCode::Char('a') => self.screen = Screen::About,
            KeyCode::Char('s') => self.screen = Screen::Stats,
            KeyCode::Char('w') => self.screen = Screen::Wikipedia,
            KeyCode::Char('n') => {
                if !self.fetching && self.typing_state != TypingState::Typing {
                    self.screen = Screen::Typing;
                    self.fetch_new_text();
                }
            }
            _ => return false,
        }
        true
    }

    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            return true;
        }
        if self.fetching {
            return false;
        }

        if ctrl && self.on_ctrl_key(key.code) {
            return false;
        }

        let typing_in_progress =
            self.screen == Screen::Typing && self.typing_state == TypingState::Typing;
        if !typing_in_progress {
            const ORDER: [Screen; 7] = [
                Screen::Typing,
                Screen::Config,
                Screen::Stats,
                Screen::Wikipedia,
                Screen::Calendar,
                Screen::About,
                Screen::Exit,
            ];
            let cur = ORDER.iter().position(|s| s == &self.screen).unwrap_or(0);
            let next = match key.code {
                KeyCode::Left => Some((cur + ORDER.len() - 1) % ORDER.len()),
                KeyCode::Right => Some((cur + 1) % ORDER.len()),
                _ => None,
            };
            if let Some(next) = next {
                self.screen = ORDER[next].clone();
                if self.screen == Screen::Calendar {
                    self.open_calendar();
                }
                if self.screen == Screen::Config {
                    self.open_config();
                }
                return false;
            }
        }

        if key.code == KeyCode::Esc {
            match self.screen {
                Screen::Config
                | Screen::Stats
                | Screen::Wikipedia
                | Screen::Calendar
                | Screen::About => {
                    self.screen = Screen::Typing;
                    return false;
                }
                Screen::Exit | Screen::Typing => return true,
            }
        }

        if self.screen == Screen::Exit && key.code == KeyCode::Enter {
            return true;
        }

        match self.screen {
            Screen::Config => self.on_key_config(key),
            Screen::Typing => self.on_key_typing(key),
            Screen::Calendar => self.on_key_calendar(key),
            Screen::Wikipedia => self.on_key_wikipedia(key),
            _ => {}
        }
        false
    }

    pub fn on_key_config(&mut self, key: KeyEvent) {
        const MODES: [TypingMode; 5] = [
            TypingMode::Forward,
            TypingMode::Stop,
            TypingMode::Correct,
            TypingMode::SuddenDeath,
            TypingMode::Blind,
        ];
        const SOURCES: [TextSource; 2] = [TextSource::Wikipedia, TextSource::WordSalad];
        const LENGTHS: [TextLength; 4] = [
            TextLength::OneLine,
            TextLength::ShortParagraph,
            TextLength::Paragraph,
            TextLength::LongParagraph,
        ];
        match key.code {
            KeyCode::Tab => {
                self.config_section = (self.config_section + 1) % 3;
            }
            KeyCode::Up => match self.config_section {
                0 => {
                    if self.config_cursor > 0 {
                        self.config_cursor -= 1;
                    }
                }
                1 => {
                    if self.config_source_cursor > 0 {
                        self.config_source_cursor -= 1;
                    }
                }
                _ => {
                    if self.config_length_cursor > 0 {
                        self.config_length_cursor -= 1;
                    }
                }
            },
            KeyCode::Down => match self.config_section {
                0 => {
                    if self.config_cursor + 1 < MODES.len() {
                        self.config_cursor += 1;
                    }
                }
                1 => {
                    if self.config_source_cursor + 1 < SOURCES.len() {
                        self.config_source_cursor += 1;
                    }
                }
                _ => {
                    if self.config_length_cursor + 1 < LENGTHS.len() {
                        self.config_length_cursor += 1;
                    }
                }
            },
            KeyCode::Enter => {
                self.config.mode = MODES[self.config_cursor];
                self.config.text_source = SOURCES[self.config_source_cursor];
                self.config.text_length = LENGTHS[self.config_length_cursor];
                save_config(&self.config);
            }
            _ => {}
        }
    }

    pub fn on_key_wikipedia(&mut self, key: KeyEvent) {
        if self.wiki_collecting {
            return;
        }
        match key.code {
            KeyCode::Char('d') | KeyCode::Enter => {
                self.start_wiki_collect(1000);
            }
            _ => {}
        }
    }

    pub fn on_key_typing(&mut self, key: KeyEvent) {
        match self.typing_state {
            TypingState::Done => match key.code {
                KeyCode::Char('n') => self.fetch_new_text(),
                KeyCode::Char('r' | ' ') | KeyCode::Enter => self.restart(),
                _ => {}
            },
            TypingState::Waiting | TypingState::Typing => {
                if self.typing_state == TypingState::Waiting {
                    if key.code == KeyCode::Char('n') {
                        self.fetch_new_text();
                        return;
                    }
                    if matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace) {
                        self.typing_state = TypingState::Typing;
                        self.start_time = Some(Instant::now());
                    }
                }

                if let Some(start) = self.start_time {
                    let offset_ms = start.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
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
                                self.last_pressed = Some((ch, ch == expected));
                            }
                            TypingMode::Stop => {
                                if ch == expected {
                                    self.typed.push(ch);
                                    self.cursor += 1;
                                } else {
                                    self.errors += 1;
                                    self.error_flash = true;
                                }
                                self.last_pressed = Some((ch, ch == expected));
                            }
                            TypingMode::SuddenDeath => {
                                if ch == expected {
                                    self.typed.push(ch);
                                    self.cursor += 1;
                                    self.last_pressed = Some((ch, true));
                                } else {
                                    // Act as if backspaced to the beginning:
                                    // clear typed text and cursor but keep
                                    // timer, stats, and keystrokes intact.
                                    self.typed.clear();
                                    self.cursor = 0;
                                    self.errors += 1;
                                    self.error_flash = true;
                                    self.last_pressed = Some((ch, false));
                                    return;
                                }
                            }
                        }

                        let all_correct = self
                            .typed
                            .iter()
                            .zip(self.target.iter())
                            .all(|(a, b)| a == b);
                        let at_end = self.cursor == self.target.len();
                        if at_end
                            && (self.config.mode != TypingMode::Correct || all_correct)
                            && let Some(start) = self.start_time
                        {
                            let elapsed = start.elapsed();
                            let minutes = elapsed.as_secs_f64() / 60.0;
                            let words = usize_to_f64(self.target.len()) / 5.0;
                            self.wpm = words / minutes;
                            self.typing_state = TypingState::Done;

                            save_session(&Session {
                                timestamp: now_timestamp(),
                                text: self.target.iter().collect(),
                                mode: format!("{:?}", self.config.mode).to_lowercase(),
                                wpm: self.wpm,
                                errors: self.errors,
                                keystrokes: self.keystrokes.clone(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub const fn open_config(&mut self) {
        self.config_section = 0;
        self.config_cursor = match self.config.mode {
            TypingMode::Forward => 0,
            TypingMode::Stop => 1,
            TypingMode::Correct => 2,
            TypingMode::SuddenDeath => 3,
            TypingMode::Blind => 4,
        };
        self.config_source_cursor = match self.config.text_source {
            TextSource::Wikipedia => 0,
            TextSource::WordSalad => 1,
        };
        self.config_length_cursor = match self.config.text_length {
            TextLength::OneLine => 0,
            TextLength::ShortParagraph => 1,
            TextLength::Paragraph => 2,
            TextLength::LongParagraph => 3,
        };
        self.screen = Screen::Config;
    }

    pub fn open_calendar(&mut self) {
        let (y, m) = today_ym();
        self.calendar_year = y;
        self.calendar_month = m;
        self.calendar_stats = load_history_stats();
        self.screen = Screen::Calendar;
    }

    pub const fn on_key_calendar(&mut self, key: KeyEvent) {
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

    pub fn accuracy(&self) -> f64 {
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
        usize_to_f64(correct) / usize_to_f64(total_keys) * 100.0
    }
}

pub fn fetch_text(source: TextSource, length: TextLength) -> String {
    match source {
        TextSource::Wikipedia => {
            pick_collected_paragraph(length).unwrap_or_else(|| FALLBACK_TEXT.to_string())
        }
        TextSource::WordSalad => generate_word_salad(length),
    }
}

/// Split `text` into display lines of at most `max_w` chars, breaking after spaces.
fn wrap_ranges(text: &[char], max_w: usize) -> Vec<(usize, usize)> {
    let mut lines_ranges: Vec<(usize, usize)> = Vec::new();
    let mut pos = 0;
    while pos < text.len() {
        let remaining = text.len() - pos;
        let take = remaining.min(max_w);
        let end = if pos + take >= text.len() {
            text.len()
        } else {
            let slice = &text[pos..pos + take];
            slice
                .iter()
                .rposition(|&c| c == ' ')
                .map_or_else(|| pos + take, |sp| pos + sp + 1)
        };
        lines_ranges.push((pos, end));
        pos = end;
    }
    lines_ranges
}

/// Styled span for the target character at index `i`.
fn char_span(app: &App, i: usize, ch: char, blind: bool) -> Span<'static> {
    match i.cmp(&app.cursor) {
        std::cmp::Ordering::Less => {
            if blind {
                Span::styled("·", Style::default().fg(Color::DarkGray))
            } else if app.typed[i] == ch {
                Span::styled(ch.to_string(), Style::default().fg(Color::Green))
            } else {
                Span::styled(
                    ch.to_string(),
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::UNDERLINED),
                )
            }
        }
        std::cmp::Ordering::Equal => {
            let bg = if app.error_flash {
                Color::Red
            } else {
                Color::White
            };
            Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(Color::Black)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            )
        }
        std::cmp::Ordering::Greater => {
            Span::styled(ch.to_string(), Style::default().fg(Color::DarkGray))
        }
    }
}

/// Live WPM / accuracy / errors / time line shown while typing.
fn live_stats_text(app: &App) -> String {
    let live_wpm = app.start_time.map_or(0, |start| {
        let mins = start.elapsed().as_secs_f64() / 60.0;
        if mins > 0.0 {
            f64_to_u32(usize_to_f64(app.cursor) / 5.0 / mins)
        } else {
            0
        }
    });
    let total_keys = app.cursor + app.errors;
    let accuracy = if total_keys > 0 {
        f64_to_u32(
            usize_to_f64(app.cursor.saturating_sub(app.errors)) / usize_to_f64(total_keys) * 100.0,
        )
    } else {
        100
    };
    let elapsed = app.start_time.map_or(0, |s| s.elapsed().as_secs());
    format!(
        "WPM: {}   accuracy: {}%   errors: {}   time: {}:{:02}",
        live_wpm,
        accuracy,
        app.errors,
        elapsed / 60,
        elapsed % 60
    )
}

pub fn render_typing(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    if app.fetching {
        let msg = Line::from(Span::styled(
            "  Fetching text from Wikipedia…",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(vec![Line::from(""), msg]), area);
        return;
    }

    let blind = app.config.mode == TypingMode::Blind;
    let max_w = usize::from(area.width).saturating_sub(4).max(20);
    let text = &app.target;
    let lines_ranges = wrap_ranges(text, max_w);

    let cursor_line = lines_ranges
        .iter()
        .position(|&(s, e)| app.cursor >= s && app.cursor < e.max(s + 1))
        .unwrap_or_else(|| lines_ranges.len().saturating_sub(1));

    let keyboard_h = 5u16;
    let reserved = 5u16 + keyboard_h + 1;
    let viewport_h = usize::from(area.height.saturating_sub(reserved));
    let viewport_h = viewport_h.max(1);

    let scroll_top = cursor_line.saturating_sub(viewport_h / 2);
    let view_y = area.y;
    for (row, &(start, end)) in lines_ranges
        .iter()
        .enumerate()
        .skip(scroll_top)
        .take(viewport_h)
    {
        let mut spans: Vec<Span> = Vec::new();
        for (i, &ch) in text.iter().enumerate().take(end).skip(start) {
            let span = char_span(app, i, ch, blind);
            spans.push(span);
        }
        let render_row = view_y + u16::try_from(row - scroll_top).unwrap_or(u16::MAX);
        if render_row < area.bottom().saturating_sub(reserved) {
            let line_rect = Rect::new(area.x, render_row, area.width, 1);
            frame.render_widget(Paragraph::new(Line::from(spans)), line_rect);
        }
    }

    let progress_pct = if app.target.is_empty() {
        0.0
    } else {
        usize_to_f64(app.cursor) / usize_to_f64(app.target.len())
    };
    let bar_w = usize::from(area.width);
    let filled = f64_to_usize(usize_to_f64(bar_w) * progress_pct);
    let empty = bar_w.saturating_sub(filled);
    let bar_text = format!(
        "[{}{}] {:.0}%",
        "█".repeat(filled),
        "░".repeat(empty),
        progress_pct * 100.0
    );
    let bar_y = area.bottom().saturating_sub(reserved - 1);
    let bar_rect = Rect::new(area.x, bar_y, area.width, 1);
    frame.render_widget(
        Paragraph::new(bar_text).style(Style::default().fg(Color::Yellow)),
        bar_rect,
    );

    if app.typing_state == TypingState::Typing {
        let stats_text = live_stats_text(app);
        let stats_rect = Rect::new(area.x, bar_y + 2, area.width, 1);
        if stats_rect.bottom() <= area.bottom() {
            frame.render_widget(
                Paragraph::new(stats_text).style(Style::default().fg(Color::Cyan)),
                stats_rect,
            );
        }
    }

    let kb_y = bar_y + 4;
    let kb_rect = Rect::new(area.x, kb_y, area.width, keyboard_h);
    if kb_rect.bottom() <= area.bottom() {
        render_keyboard(frame, kb_rect, app);
    }

    let mid_session = app.typing_state == TypingState::Typing;
    let key_style = if mid_session {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    };
    let hint = Line::from(vec![
        Span::styled("  ^", Style::default().fg(Color::DarkGray)),
        Span::styled("N", key_style),
        Span::styled(" new text", Style::default().fg(Color::DarkGray)),
    ]);
    let hint_y = area.bottom().saturating_sub(1);
    frame.render_widget(
        Paragraph::new(hint),
        Rect::new(area.x, hint_y, area.width, 1),
    );
}

pub const fn base_key(c: char) -> char {
    match c {
        '~' => '`',
        '!' => '1',
        '@' => '2',
        '#' => '3',
        '$' => '4',
        '%' => '5',
        '^' => '6',
        '&' => '7',
        '*' => '8',
        '(' => '9',
        ')' => '0',
        '_' => '-',
        '+' => '=',
        '{' => '[',
        '}' => ']',
        '|' => '\\',
        ':' => ';',
        '"' => '\'',
        '<' => ',',
        '>' => '.',
        '?' => '/',
        c => c.to_ascii_lowercase(),
    }
}

/// Style of one keyboard cell given the pressed and expected keys.
fn key_cell_style(pressed: Option<char>, expected: Option<char>, k: char, correct: bool) -> Style {
    let base = Style::default()
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);
    if pressed == Some(k) {
        if correct {
            base.bg(Color::Green)
        } else {
            base.bg(Color::Red)
        }
    } else if expected == Some(k) {
        base.bg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// Style of the L/R hand label: highlighted when the last pressed key used that hand.
fn hand_label_style(app: &App, hand: Hand) -> Style {
    if app
        .last_pressed
        .is_some_and(|(c, _)| hand_for_char(c) == Some(hand))
    {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub fn render_keyboard(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    const ROWS: &[&[char]] = &[
        &[
            '`', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=',
        ],
        &[
            'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', '\\',
        ],
        &['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\''],
        &['z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/'],
    ];
    const OFFSETS: &[u16] = &[0, 1, 2, 3];
    const CELL_W: u16 = 3;

    let expected_base = if app.cursor < app.target.len() {
        Some(base_key(app.target[app.cursor]))
    } else {
        None
    };
    let pressed_base = app.last_pressed.map(|(c, _)| base_key(c));

    let correct = app.last_pressed.is_some_and(|(_, ok)| ok);

    for (row_idx, keys) in ROWS.iter().enumerate() {
        let row_u16 = u16::try_from(row_idx).unwrap_or(u16::MAX);
        if row_u16 >= area.height {
            break;
        }
        let y = area.y + row_u16;
        let offset = OFFSETS[row_idx];
        let mut spans: Vec<Span> = Vec::new();
        if offset > 0 {
            spans.push(Span::raw(" ".repeat(usize::from(offset))));
        }
        for &k in *keys {
            let display = if k == '\\' {
                "\\ ".to_string()
            } else {
                format!(" {k} ")
            };
            let style = key_cell_style(pressed_base, expected_base, k, correct);
            spans.push(Span::styled(display, style));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(area.x, y, area.width, 1),
        );
    }

    if 4 < area.height {
        let y = area.y + 4;
        let style = key_cell_style(pressed_base, expected_base, ' ', correct);
        let mut spans = vec![
            Span::raw(" ".repeat(usize::from(OFFSETS[3]) + usize::from(CELL_W) * 2)),
            Span::styled("   space   ", style),
            Span::raw("  "),
        ];
        let left_label_style = hand_label_style(app, Hand::Left);
        let right_label_style = hand_label_style(app, Hand::Right);
        spans.push(Span::styled("L", left_label_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("R", right_label_style));
        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(area.x, y, area.width, 1),
        );
    }
}

/// Left/right hand comparison table shown on the results screen.
fn hand_report_lines(left_stats: &HandStats, right_stats: &HandStats) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "── Hand Report ──",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<14}", ""), Style::default()),
        Span::styled(
            format!("{:>10}", "Left"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:>10}", "Right"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    let left_resp = if left_stats.total_keys > 1 {
        format!("{:.0} ms", left_stats.avg_response_ms)
    } else {
        "—".to_string()
    };
    let right_resp = if right_stats.total_keys > 1 {
        format!("{:.0} ms", right_stats.avg_response_ms)
    } else {
        "—".to_string()
    };
    lines.push(Line::from(vec![
        Span::raw(format!("{:<14}", "Avg response")),
        Span::styled(
            format!("{left_resp:>10}"),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!("{right_resp:>10}"),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw(format!("{:<14}", "Keys typed")),
        Span::styled(
            format!("{:>10}", left_stats.total_keys),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!("{:>10}", right_stats.total_keys),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw(format!("{:<14}", "Errors")),
        Span::styled(
            format!("{:>10}", left_stats.errors),
            Style::default().fg(if left_stats.errors == 0 {
                Color::Green
            } else {
                Color::Red
            }),
        ),
        Span::styled(
            format!("{:>10}", right_stats.errors),
            Style::default().fg(if right_stats.errors == 0 {
                Color::Green
            } else {
                Color::Red
            }),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw(format!("{:<14}", "Error rate")),
        Span::styled(
            format!("{:>9.1}%", left_stats.error_rate()),
            Style::default().fg(if left_stats.error_rate() < 5.0 {
                Color::Green
            } else {
                Color::Red
            }),
        ),
        Span::styled(
            format!("{:>9.1}%", right_stats.error_rate()),
            Style::default().fg(if right_stats.error_rate() < 5.0 {
                Color::Green
            } else {
                Color::Red
            }),
        ),
    ]));
    lines
}

pub fn render_done(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let accuracy = app.accuracy();
    let acc_color = if accuracy >= 95.0 {
        Color::Green
    } else if accuracy >= 80.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    let (left_stats, right_stats) = compute_hand_stats(&app.target, &app.typed, &app.keystrokes);

    let mut lines = vec![
        Line::from(Span::styled(
            "── Results ──",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("Speed:    "),
            Span::styled(
                format!("{:.1} WPM", app.wpm),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Accuracy: "),
            Span::styled(
                format!("{accuracy:.1}%"),
                Style::default().fg(acc_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Errors:   "),
            Span::styled(
                format!("{}", app.errors),
                Style::default()
                    .fg(if app.errors == 0 {
                        Color::Green
                    } else {
                        Color::Red
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    if left_stats.total_keys > 0 || right_stats.total_keys > 0 {
        lines.extend(hand_report_lines(&left_stats, &right_stats));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Enter / Space / R  retry same text     N  new text",
        Style::default().fg(Color::DarkGray),
    )));

    let result_rect = centered_rect(52, u16::try_from(lines.len()).unwrap_or(u16::MAX), area);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        result_rect,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{days_in_month_cal, days_to_ymd, first_weekday_of_month, is_leap_year};
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
        assert_eq!(days_in_month_cal(2024, 4), 30); // April
        assert_eq!(days_in_month_cal(2024, 1), 31); // January
        assert_eq!(days_in_month_cal(2024, 12), 31); // December
    }

    #[test]
    fn first_weekday_known_date() {
        assert_eq!(first_weekday_of_month(2024, 1), 0);
        assert_eq!(first_weekday_of_month(2024, 4), 0);
        assert_eq!(first_weekday_of_month(2024, 3), 4);
    }

    #[test]
    fn days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_known() {
        assert_eq!(days_to_ymd(19723), (2024, 1, 1));
    }

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

    #[test]
    fn mode_labels_distinct() {
        let labels: Vec<_> = [
            TypingMode::Forward,
            TypingMode::Stop,
            TypingMode::Correct,
            TypingMode::SuddenDeath,
            TypingMode::Blind,
        ]
        .iter()
        .map(|m| m.label())
        .collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len());
    }

    #[test]
    fn mode_descriptions_non_empty() {
        for m in [
            TypingMode::Forward,
            TypingMode::Stop,
            TypingMode::Correct,
            TypingMode::SuddenDeath,
            TypingMode::Blind,
        ] {
            assert_ne!(m.description(), "");
        }
    }

    fn app_with_text(text: &str, mode: TypingMode) -> App {
        let config = Config {
            mode,
            ..Default::default()
        };
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
        app.on_key(key(KeyCode::Char('x')));
        app.on_key(key(KeyCode::Char('b')));
        assert_ne!(app.typing_state, TypingState::Done);
    }

    #[test]
    fn correct_mode_finishes_when_all_correct() {
        let mut app = app_with_text("ab", TypingMode::Correct);
        app.on_key(key(KeyCode::Char('x')));
        app.on_key(key(KeyCode::Backspace));
        app.on_key(key(KeyCode::Char('a')));
        app.on_key(key(KeyCode::Char('b')));
        assert_eq!(app.typing_state, TypingState::Done);
    }

    #[test]
    fn sudden_death_resets_on_wrong_key() {
        let mut app = app_with_text("abc", TypingMode::SuddenDeath);
        app.on_key(key(KeyCode::Char('a'))); // correct — starts timer
        assert_eq!(app.typing_state, TypingState::Typing);
        app.on_key(key(KeyCode::Char('x'))); // wrong — resets cursor but keeps state
        assert_eq!(app.cursor, 0);
        assert_eq!(app.typed.len(), 0);
        assert_eq!(app.typing_state, TypingState::Typing);
        assert!(app.start_time.is_some());
        assert_eq!(app.errors, 1);
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

    #[test]
    fn hand_for_left_keys() {
        for c in [
            'q', 'w', 'e', 'r', 't', 'a', 's', 'd', 'f', 'g', 'z', 'x', 'c', 'v', 'b',
        ] {
            assert_eq!(
                hand_for_char(c),
                Some(Hand::Left),
                "expected Left for '{c}'"
            );
        }
    }

    #[test]
    fn hand_for_right_keys() {
        for c in ['y', 'u', 'i', 'o', 'p', 'h', 'j', 'k', 'l', 'n', 'm'] {
            assert_eq!(
                hand_for_char(c),
                Some(Hand::Right),
                "expected Right for '{c}'"
            );
        }
    }

    #[test]
    fn hand_for_space_is_none() {
        assert_eq!(hand_for_char(' '), None);
    }

    #[test]
    fn hand_for_uppercase_same_as_lower() {
        assert_eq!(hand_for_char('Q'), Some(Hand::Left));
        assert_eq!(hand_for_char('P'), Some(Hand::Right));
    }

    #[test]
    fn hand_stats_all_correct() {
        let target: Vec<char> = "ash".chars().collect();
        let typed: Vec<char> = "ash".chars().collect();
        let keystrokes = vec![
            Keystroke {
                typed: "a".into(),
                offset_ms: 0,
            },
            Keystroke {
                typed: "s".into(),
                offset_ms: 100,
            },
            Keystroke {
                typed: "h".into(),
                offset_ms: 250,
            },
        ];
        let (left, right) = compute_hand_stats(&target, &typed, &keystrokes);
        assert_eq!(left.total_keys, 2);
        assert_eq!(left.errors, 0);
        assert_eq!(right.total_keys, 1);
        assert_eq!(right.errors, 0);
    }

    #[test]
    fn hand_stats_with_errors() {
        let target: Vec<char> = "ash".chars().collect();
        let typed: Vec<char> = "xsh".chars().collect();
        let keystrokes = vec![
            Keystroke {
                typed: "x".into(),
                offset_ms: 0,
            },
            Keystroke {
                typed: "s".into(),
                offset_ms: 100,
            },
            Keystroke {
                typed: "h".into(),
                offset_ms: 200,
            },
        ];
        let (left, right) = compute_hand_stats(&target, &typed, &keystrokes);
        assert_eq!(left.total_keys, 2);
        assert_eq!(left.errors, 1);
        assert_eq!(right.total_keys, 1);
        assert_eq!(right.errors, 0);
        assert!(left.error_rate() > 0.0);
        assert_eq!(right.error_rate(), 0.0);
    }

    #[test]
    fn hand_stats_response_times() {
        let target: Vec<char> = "ah".chars().collect();
        let typed: Vec<char> = "ah".chars().collect();
        let keystrokes = vec![
            Keystroke {
                typed: "a".into(),
                offset_ms: 0,
            },
            Keystroke {
                typed: "h".into(),
                offset_ms: 200,
            },
        ];
        let (left, right) = compute_hand_stats(&target, &typed, &keystrokes);
        assert_eq!(left.avg_response_ms, 0.0);
        assert_eq!(right.avg_response_ms, 0.0);
    }

    #[test]
    fn hand_stats_avg_response_with_enough_keys() {
        let target: Vec<char> = "asdf".chars().collect();
        let typed: Vec<char> = "asdf".chars().collect();
        let keystrokes = vec![
            Keystroke {
                typed: "a".into(),
                offset_ms: 0,
            },
            Keystroke {
                typed: "s".into(),
                offset_ms: 100,
            },
            Keystroke {
                typed: "d".into(),
                offset_ms: 250,
            },
            Keystroke {
                typed: "f".into(),
                offset_ms: 400,
            },
        ];
        let (left, _right) = compute_hand_stats(&target, &typed, &keystrokes);
        assert_eq!(left.total_keys, 4);
        let expected_avg = (100.0 + 150.0 + 150.0) / 3.0;
        assert!((left.avg_response_ms - expected_avg).abs() < 1.0);
    }

    #[test]
    fn word_salad_respects_length_bounds() {
        for length in [
            TextLength::OneLine,
            TextLength::ShortParagraph,
            TextLength::Paragraph,
            TextLength::LongParagraph,
        ] {
            let text = generate_word_salad(length);
            assert!(
                text.len() >= length.min_chars(),
                "word salad too short for {:?}: {} < {}",
                length,
                text.len(),
                length.min_chars()
            );
            assert!(
                text.len() <= length.max_chars(),
                "word salad too long for {:?}: {} > {}",
                length,
                text.len(),
                length.max_chars()
            );
        }
    }

    #[test]
    fn word_salad_only_ascii_lowercase_and_spaces() {
        let text = generate_word_salad(TextLength::Paragraph);
        assert!(text.chars().all(|c| c.is_ascii_lowercase() || c == ' '));
    }
}
