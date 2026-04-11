use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{Duration, Instant};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::utils::*;
use crate::wikipedia::{fetch_wikipedia_paragraphs_batch, load_paragraphs, pick_collected_paragraph, WikiCollectMsg};
use crate::dict::generate_word_salad;

#[derive(PartialEq, Debug)]
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
    pub last_pressed_key: Option<char>,
    pub last_pressed_correct: bool,
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
            last_pressed_key: None,
            last_pressed_correct: false,
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
        self.last_pressed_key = None;
        self.last_pressed_correct = false;
        self.typing_state = TypingState::Waiting;
        self.start_time = None;
        self.wpm = 0.0;
        self.keystrokes = Vec::new();
        self.fetching = true;
        self.fetch_rx = Some(rx);
    }

    pub fn poll_fetch(&mut self) {
        if let Some(rx) = &self.fetch_rx {
            if let Ok(text) = rx.try_recv() {
                self.target = text.chars().collect();
                self.fetching = false;
                self.fetch_rx = None;
            }
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
        if self.wiki_collecting { return; }
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

            let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
                Ok(f) => f,
                Err(_) => {
                    let _ = tx.send(WikiCollectMsg::Done(total));
                    return;
                }
            };

            let mut requests = 0u32;
            while total < target {
                requests += 1;
                let batch = fetch_wikipedia_paragraphs_batch();
                for para in batch {
                    if seen.contains(&para) { continue; }
                    seen.insert(para.clone());
                    if let Ok(json) = serde_json::to_string(&serde_json::json!({ "text": para })) {
                        let _ = writeln!(file, "{}", json);
                        total += 1;
                    }
                    if total >= target { break; }
                }
                let _ = tx.send(WikiCollectMsg::Progress(total, requests));
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = tx.send(WikiCollectMsg::Done(total));
        });
    }

    pub fn restart(&mut self) {
        let cfg = self.config.clone();
        *self = App::new(cfg);
    }

    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') { return true; }
        if self.fetching { return false; }

        if ctrl {
            match key.code {
                KeyCode::Char('e') => { self.screen = Screen::Exit; return false; }
                KeyCode::Char('t') => { self.screen = Screen::Typing; return false; }
                KeyCode::Char('g') => { self.open_config(); return false; }
                KeyCode::Char('h') => { self.open_calendar(); return false; }
                KeyCode::Char('a') => { self.screen = Screen::About; return false; }
                KeyCode::Char('s') => { self.screen = Screen::Stats; return false; }
                KeyCode::Char('w') => { self.screen = Screen::Wikipedia; return false; }
                KeyCode::Char('n') => {
                    if !self.fetching && self.typing_state != TypingState::Typing {
                        self.screen = Screen::Typing;
                        self.fetch_new_text();
                    }
                    return false;
                }
                _ => {}
            }
        }

        let typing_in_progress = self.screen == Screen::Typing && self.typing_state == TypingState::Typing;
        if !typing_in_progress {
            const ORDER: [Screen; 7] = [Screen::Typing, Screen::Config, Screen::Stats, Screen::Wikipedia, Screen::Calendar, Screen::About, Screen::Exit];
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

        if key.code == KeyCode::Esc {
            match self.screen {
                Screen::Config | Screen::Stats | Screen::Wikipedia | Screen::Calendar | Screen::About => {
                    self.screen = Screen::Typing;
                    return false;
                }
                Screen::Exit | Screen::Typing => return true,
            }
        }

        if self.screen == Screen::Exit && key.code == KeyCode::Enter { return true; }

        match self.screen {
            Screen::Config    => self.on_key_config(key),
            Screen::Typing    => self.on_key_typing(key),
            Screen::Calendar  => self.on_key_calendar(key),
            Screen::Wikipedia => self.on_key_wikipedia(key),
            _ => {}
        }
        false
    }

    pub fn on_key_config(&mut self, key: KeyEvent) {
        const MODES: [TypingMode; 5] = [TypingMode::Forward, TypingMode::Stop, TypingMode::Correct, TypingMode::SuddenDeath, TypingMode::Blind];
        const SOURCES: [TextSource; 2] = [TextSource::Wikipedia, TextSource::WordSalad];
        const LENGTHS: [TextLength; 4] = [TextLength::OneLine, TextLength::ShortParagraph, TextLength::Paragraph, TextLength::LongParagraph];
        match key.code {
            KeyCode::Tab => { self.config_section = (self.config_section + 1) % 3; }
            KeyCode::Up => {
                match self.config_section {
                    0 => { if self.config_cursor > 0 { self.config_cursor -= 1; } }
                    1 => { if self.config_source_cursor > 0 { self.config_source_cursor -= 1; } }
                    _ => { if self.config_length_cursor > 0 { self.config_length_cursor -= 1; } }
                }
            }
            KeyCode::Down => {
                match self.config_section {
                    0 => { if self.config_cursor + 1 < MODES.len() { self.config_cursor += 1; } }
                    1 => { if self.config_source_cursor + 1 < SOURCES.len() { self.config_source_cursor += 1; } }
                    _ => { if self.config_length_cursor + 1 < LENGTHS.len() { self.config_length_cursor += 1; } }
                }
            }
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
        if self.wiki_collecting { return; }
        match key.code {
            KeyCode::Char('d') | KeyCode::Enter => { self.start_wiki_collect(1000); }
            _ => {}
        }
    }

    pub fn on_key_typing(&mut self, key: KeyEvent) {
        match self.typing_state {
            TypingState::Done => {
                match key.code {
                    KeyCode::Char('n') => self.fetch_new_text(),
                    KeyCode::Char('r') | KeyCode::Enter | KeyCode::Char(' ') => self.restart(),
                    _ => {}
                }
            }
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
                                if ch != expected { self.errors += 1; }
                                self.last_pressed_key = Some(ch);
                                self.last_pressed_correct = ch == expected;
                            }
                            TypingMode::Stop => {
                                if ch == expected {
                                    self.typed.push(ch);
                                    self.cursor += 1;
                                } else {
                                    self.errors += 1;
                                    self.error_flash = true;
                                }
                                self.last_pressed_key = Some(ch);
                                self.last_pressed_correct = ch == expected;
                            }
                            TypingMode::SuddenDeath => {
                                if ch == expected {
                                    self.typed.push(ch);
                                    self.cursor += 1;
                                    self.last_pressed_key = Some(ch);
                                    self.last_pressed_correct = true;
                                } else {
                                    // Act as if backspaced to the beginning:
                                    // clear typed text and cursor but keep
                                    // timer, stats, and keystrokes intact.
                                    self.typed.clear();
                                    self.cursor = 0;
                                    self.errors += 1;
                                    self.error_flash = true;
                                    self.last_pressed_key = Some(ch);
                                    self.last_pressed_correct = false;
                                    return;
                                }
                            }
                        }

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
                                keystrokes: self.keystrokes.clone(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn open_config(&mut self) {
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
        self.config_length_cursor = match self.config.text_length {
            TextLength::OneLine        => 0,
            TextLength::ShortParagraph => 1,
            TextLength::Paragraph      => 2,
            TextLength::LongParagraph  => 3,
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

    pub fn on_key_calendar(&mut self, key: KeyEvent) {
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
        if total_keys == 0 { return 100.0; }
        let correct = self.typed.iter().zip(self.target.iter()).filter(|(t, r)| t == r).count();
        correct as f64 / total_keys as f64 * 100.0
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

pub fn render_typing(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    if app.fetching {
        let msg = Line::from(Span::styled(
            "  Fetching text from Wikipedia…",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(vec![Line::from(""), msg]), area);
        return;
    }

    let blind = app.config.mode == TypingMode::Blind;
    let max_w = (area.width as usize).saturating_sub(4).max(20);
    let mut lines_ranges: Vec<(usize, usize)> = Vec::new();
    let text = &app.target;
    let mut pos = 0;
    while pos < text.len() {
        let remaining = text.len() - pos;
        let take = remaining.min(max_w);
        let end = if pos + take >= text.len() {
            text.len()
        } else {
            let slice = &text[pos..pos + take];
            if let Some(sp) = slice.iter().rposition(|&c| c == ' ') {
                pos + sp + 1
            } else {
                pos + take
            }
        };
        lines_ranges.push((pos, end));
        pos = end;
    }

    let cursor_line = lines_ranges
        .iter()
        .position(|&(s, e)| app.cursor >= s && app.cursor < e.max(s + 1))
        .unwrap_or(lines_ranges.len().saturating_sub(1));

    let keyboard_h = 5u16;
    let reserved = 5u16 + keyboard_h + 1;
    let viewport_h = area.height.saturating_sub(reserved) as usize;
    let viewport_h = viewport_h.max(1);

    let scroll_top = if cursor_line < viewport_h / 2 { 0 } else { cursor_line - viewport_h / 2 };
    let view_y = area.y;
    for (row, &(start, end)) in lines_ranges.iter().enumerate().skip(scroll_top).take(viewport_h) {
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

    let progress_pct = if app.target.is_empty() { 0.0 } else { app.cursor as f64 / app.target.len() as f64 };
    let bar_w = area.width as usize;
    let filled = (bar_w as f64 * progress_pct) as usize;
    let empty = bar_w.saturating_sub(filled);
    let bar_text = format!("[{}{}] {:.0}%", "█".repeat(filled), "░".repeat(empty), progress_pct * 100.0);
    let bar_y = area.bottom().saturating_sub(reserved - 1);
    let bar_rect = Rect::new(area.x, bar_y, area.width, 1);
    frame.render_widget(Paragraph::new(bar_text).style(Style::default().fg(Color::Yellow)), bar_rect);

    if app.typing_state == TypingState::Typing {
        let live_wpm = if let Some(start) = app.start_time {
            let mins = start.elapsed().as_secs_f64() / 60.0;
            if mins > 0.0 { (app.cursor as f64 / 5.0 / mins) as u32 } else { 0 }
        } else { 0 };
        let total_keys = app.cursor + app.errors;
        let accuracy = if total_keys > 0 { (app.cursor.saturating_sub(app.errors) as f64 / total_keys as f64 * 100.0) as u32 } else { 100 };
        let elapsed = app.start_time.map(|s| s.elapsed().as_secs()).unwrap_or(0);
        let stats_text = format!("WPM: {}   accuracy: {}%   errors: {}   time: {}:{:02}", live_wpm, accuracy, app.errors, elapsed / 60, elapsed % 60);
        let stats_rect = Rect::new(area.x, bar_y + 2, area.width, 1);
        if stats_rect.bottom() <= area.bottom() {
            frame.render_widget(Paragraph::new(stats_text).style(Style::default().fg(Color::Cyan)), stats_rect);
        }
    }

    let kb_y = bar_y + 4;
    let kb_rect = Rect::new(area.x, kb_y, area.width, keyboard_h);
    if kb_rect.bottom() <= area.bottom() { render_keyboard(frame, kb_rect, app); }

    let mid_session = app.typing_state == TypingState::Typing;
    let key_style = if mid_session { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) };
    let hint = Line::from(vec![
        Span::styled("  ^", Style::default().fg(Color::DarkGray)),
        Span::styled("N", key_style),
        Span::styled(" new text", Style::default().fg(Color::DarkGray)),
    ]);
    let hint_y = area.bottom().saturating_sub(1);
    frame.render_widget(Paragraph::new(hint), Rect::new(area.x, hint_y, area.width, 1));
}

pub fn base_key(c: char) -> char {
    match c {
        '~' => '`', '!' => '1', '@' => '2', '#' => '3', '$' => '4',
        '%' => '5', '^' => '6', '&' => '7', '*' => '8', '(' => '9',
        ')' => '0', '_' => '-', '+' => '=',
        '{' => '[', '}' => ']', '|' => '\\',
        ':' => ';', '"' => '\'',
        '<' => ',', '>' => '.', '?' => '/',
        c => c.to_ascii_lowercase(),
    }
}

pub fn render_keyboard(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    const ROWS: &[&[char]] = &[
        &['`','1','2','3','4','5','6','7','8','9','0','-','='],
        &['q','w','e','r','t','y','u','i','o','p','[',']','\\'],
        &['a','s','d','f','g','h','j','k','l',';','\''],
        &['z','x','c','v','b','n','m',',','.','/'],
    ];
    const OFFSETS: &[u16] = &[0, 1, 2, 3];
    const CELL_W: u16 = 3;

    let expected_base = if app.cursor < app.target.len() { Some(base_key(app.target[app.cursor])) } else { None };
    let pressed_base = app.last_pressed_key.map(base_key);

    let dim_style = Style::default().fg(Color::DarkGray);
    let expect_style = Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD);
    let correct_style = Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD);
    let wrong_style = Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD);

    for (row_idx, keys) in ROWS.iter().enumerate() {
        if row_idx as u16 >= area.height { break; }
        let y = area.y + row_idx as u16;
        let offset = OFFSETS[row_idx];
        let mut spans: Vec<Span> = Vec::new();
        if offset > 0 { spans.push(Span::raw(" ".repeat(offset as usize))); }
        for &k in *keys {
            let display = if k == '\\' { "\\ ".to_string() } else { format!(" {} ", k) };
            let style = if pressed_base == Some(k) {
                if app.last_pressed_correct { correct_style } else { wrong_style }
            } else if expected_base == Some(k) { expect_style } else { dim_style };
            spans.push(Span::styled(display, style));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), Rect::new(area.x, y, area.width, 1));
    }

    if 4 < area.height {
        let y = area.y + 4;
        let style = if pressed_base == Some(' ') {
            if app.last_pressed_correct { correct_style } else { wrong_style }
        } else if expected_base == Some(' ') { expect_style } else { dim_style };
        let mut spans = vec![
            Span::raw(" ".repeat(OFFSETS[3] as usize + CELL_W as usize * 2)),
            Span::styled("   space   ", style),
            Span::raw("  "),
        ];
        let left_label_style = if app.last_pressed_key.map(|c| hand_for_char(c) == Some(Hand::Left)).unwrap_or(false) {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else { Style::default().fg(Color::DarkGray) };
        let right_label_style = if app.last_pressed_key.map(|c| hand_for_char(c) == Some(Hand::Right)).unwrap_or(false) {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else { Style::default().fg(Color::DarkGray) };
        spans.push(Span::styled("L", left_label_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("R", right_label_style));
        frame.render_widget(Paragraph::new(Line::from(spans)), Rect::new(area.x, y, area.width, 1));
    }
}

pub fn render_done(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let accuracy = app.accuracy();
    let acc_color = if accuracy >= 95.0 { Color::Green } else if accuracy >= 80.0 { Color::Yellow } else { Color::Red };
    let (left_stats, right_stats) = compute_hand_stats(&app.target, &app.typed, &app.keystrokes);

    let mut lines = vec![
        Line::from(Span::styled("── Results ──", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::raw("Speed:    "), Span::styled(format!("{:.1} WPM", app.wpm), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::raw("Accuracy: "), Span::styled(format!("{:.1}%", accuracy), Style::default().fg(acc_color).add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::raw("Errors:   "), Span::styled(format!("{}", app.errors), Style::default().fg(if app.errors == 0 { Color::Green } else { Color::Red }).add_modifier(Modifier::BOLD))]),
    ];

    if left_stats.total_keys > 0 || right_stats.total_keys > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("── Hand Report ──", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(format!("{:<14}", ""), Style::default()),
            Span::styled(format!("{:>10}", "Left"), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>10}", "Right"), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
        let left_resp = if left_stats.total_keys > 1 { format!("{:.0} ms", left_stats.avg_response_ms) } else { "—".to_string() };
        let right_resp = if right_stats.total_keys > 1 { format!("{:.0} ms", right_stats.avg_response_ms) } else { "—".to_string() };
        lines.push(Line::from(vec![Span::raw(format!("{:<14}", "Avg response")), Span::styled(format!("{:>10}", left_resp), Style::default().fg(Color::White)), Span::styled(format!("{:>10}", right_resp), Style::default().fg(Color::White))]));
        lines.push(Line::from(vec![Span::raw(format!("{:<14}", "Keys typed")), Span::styled(format!("{:>10}", left_stats.total_keys), Style::default().fg(Color::White)), Span::styled(format!("{:>10}", right_stats.total_keys), Style::default().fg(Color::White))]));
        lines.push(Line::from(vec![Span::raw(format!("{:<14}", "Errors")), Span::styled(format!("{:>10}", left_stats.errors), Style::default().fg(if left_stats.errors == 0 { Color::Green } else { Color::Red })), Span::styled(format!("{:>10}", right_stats.errors), Style::default().fg(if right_stats.errors == 0 { Color::Green } else { Color::Red }))]));
        lines.push(Line::from(vec![Span::raw(format!("{:<14}", "Error rate")), Span::styled(format!("{:>9.1}%", left_stats.error_rate()), Style::default().fg(if left_stats.error_rate() < 5.0 { Color::Green } else { Color::Red })), Span::styled(format!("{:>9.1}%", right_stats.error_rate()), Style::default().fg(if right_stats.error_rate() < 5.0 { Color::Green } else { Color::Red }))]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Enter / Space / R  retry same text     N  new text", Style::default().fg(Color::DarkGray))));

    let result_rect = centered_rect(52, lines.len() as u16, area);
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), result_rect);
}

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
        for c in ['q', 'w', 'e', 'r', 't', 'a', 's', 'd', 'f', 'g', 'z', 'x', 'c', 'v', 'b'] {
            assert_eq!(hand_for_char(c), Some(Hand::Left), "expected Left for '{c}'");
        }
    }

    #[test]
    fn hand_for_right_keys() {
        for c in ['y', 'u', 'i', 'o', 'p', 'h', 'j', 'k', 'l', 'n', 'm'] {
            assert_eq!(hand_for_char(c), Some(Hand::Right), "expected Right for '{c}'");
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
            Keystroke { typed: "a".into(), offset_ms: 0 },
            Keystroke { typed: "s".into(), offset_ms: 100 },
            Keystroke { typed: "h".into(), offset_ms: 250 },
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
            Keystroke { typed: "x".into(), offset_ms: 0 },
            Keystroke { typed: "s".into(), offset_ms: 100 },
            Keystroke { typed: "h".into(), offset_ms: 200 },
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
            Keystroke { typed: "a".into(), offset_ms: 0 },
            Keystroke { typed: "h".into(), offset_ms: 200 },
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
            Keystroke { typed: "a".into(), offset_ms: 0 },
            Keystroke { typed: "s".into(), offset_ms: 100 },
            Keystroke { typed: "d".into(), offset_ms: 250 },
            Keystroke { typed: "f".into(), offset_ms: 400 },
        ];
        let (left, _right) = compute_hand_stats(&target, &typed, &keystrokes);
        assert_eq!(left.total_keys, 4);
        let expected_avg = (100.0 + 150.0 + 150.0) / 3.0;
        assert!((left.avg_response_ms - expected_avg).abs() < 1.0);
    }

    #[test]
    fn word_salad_respects_length_bounds() {
        for length in [TextLength::OneLine, TextLength::ShortParagraph, TextLength::Paragraph, TextLength::LongParagraph] {
            let text = generate_word_salad(length);
            assert!(text.len() >= length.min_chars(),
                "word salad too short for {:?}: {} < {}", length, text.len(), length.min_chars());
            assert!(text.len() <= length.max_chars(),
                "word salad too long for {:?}: {} > {}", length, text.len(), length.max_chars());
        }
    }

    #[test]
    fn word_salad_only_ascii_lowercase_and_spaces() {
        let text = generate_word_salad(TextLength::Paragraph);
        assert!(text.chars().all(|c| c.is_ascii_lowercase() || c == ' '));
    }
}
