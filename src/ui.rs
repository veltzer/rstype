use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use std::io;

use crate::train::{App, TypingState};
use crate::utils::{
    HandStats, Screen, TextLength, TextSource, TypingMode, compute_hand_stats, days_in_month_cal,
    f64_to_usize, first_weekday_of_month, load_all_wpms, load_history_hand_stats,
    load_history_stats, month_name, paragraphs_path, today_ym, u64_to_f64, usize_to_f64,
};
use crate::wikipedia::load_paragraphs;

pub fn render(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &App) -> io::Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            area,
        );

        let toolbar_rect = Rect::new(area.x, area.y, area.width, 1);
        let statusbar_rect = Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width,
            1,
        );
        let body_rect = Rect::new(
            area.x,
            area.y + 2,
            area.width,
            area.height.saturating_sub(3),
        );

        render_toolbar(frame, toolbar_rect, app);
        render_statusbar(frame, statusbar_rect, app);

        let indent = 2u16;
        let indented = Rect::new(
            body_rect.x + indent,
            body_rect.y,
            body_rect.width.saturating_sub(indent),
            body_rect.height,
        );

        match app.screen {
            Screen::Config => render_config(frame, body_rect, app),
            Screen::Stats => render_stats(frame, indented, app),
            Screen::Wikipedia => render_wikipedia(frame, indented, app),
            Screen::Calendar => render_calendar(frame, indented, app),
            Screen::About => render_about(frame, indented),
            Screen::Exit => render_exit(frame, indented),
            Screen::Typing => match app.typing_state {
                TypingState::Done => crate::train::render_done(frame, body_rect, app),
                _ => crate::train::render_typing(frame, indented, app),
            },
        }
    })?;
    Ok(())
}

fn render_toolbar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let active_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default().fg(Color::Black).bg(Color::White);
    let key_style = Style::default()
        .fg(Color::Blue)
        .bg(Color::White)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let active_key_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let sep_style = Style::default().fg(Color::DarkGray).bg(Color::White);

    let is_train = app.screen == Screen::Typing;
    let is_config = app.screen == Screen::Config;
    let is_stats = app.screen == Screen::Stats;
    let is_wiki = app.screen == Screen::Wikipedia;
    let is_calendar = app.screen == Screen::Calendar;
    let is_about = app.screen == Screen::About;
    let is_exit = app.screen == Screen::Exit;

    let mut spans = vec![Span::styled("  ", normal_style)];

    for (before, key, after, active) in [
        ("", 'T', "rain", is_train),
        ("Confi", 'G', "", is_config),
        ("", 'S', "tats", is_stats),
        ("", 'W', "ikipedia", is_wiki),
        ("", 'H', "istory", is_calendar),
        ("", 'A', "bout", is_about),
        ("", 'E', "xit", is_exit),
    ] {
        let (ks, rs) = if active {
            (active_key_style, active_style)
        } else {
            (key_style, normal_style)
        };
        if !before.is_empty() {
            spans.push(Span::styled(before, rs));
        }
        spans.push(Span::styled(key.to_string(), ks));
        if !after.is_empty() {
            spans.push(Span::styled(after, rs));
        }
        spans.push(Span::styled("  ", sep_style));
    }

    let left_width: u16 = 60;
    let title = "rstype by Mark Veltzer  ";
    let title_len = u16::try_from(title.len()).unwrap_or(u16::MAX);
    let pad = area.width.saturating_sub(left_width + title_len);
    spans.push(Span::styled(" ".repeat(usize::from(pad)), normal_style));
    spans.push(Span::styled(
        title,
        Style::default().fg(Color::DarkGray).bg(Color::White),
    ));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_statusbar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let style = Style::default().fg(Color::Black).bg(Color::White);
    let mode_label = match app.config.mode {
        TypingMode::Forward => "forward",
        TypingMode::Stop => "stop",
        TypingMode::Correct => "correct",
        TypingMode::SuddenDeath => "sudden death",
        TypingMode::Blind => "blind",
    };
    let text = format!(" mode: {mode_label}{}", " ".repeat(usize::from(area.width)));
    frame.render_widget(Paragraph::new(text).style(style), area);
}

const CELL: usize = 10;

fn render_calendar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let year = app.calendar_year;
    let month = app.calendar_month;
    let first_dow = first_weekday_of_month(year, month);
    let num_days = days_in_month_cal(year, month);
    let dow_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    let header_spans: Vec<Span> = dow_names
        .iter()
        .map(|n| {
            Span::styled(
                format!("{n:<CELL$}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();

    let mut lines: Vec<Line> = vec![Line::from(""), Line::from(header_spans), Line::from("")];

    let total_slots = 6 * 7;
    let mut day_spans: Vec<Span> = Vec::new();
    let mut stat_spans: Vec<Span> = Vec::new();
    let mut wc_spans: Vec<Span> = Vec::new();

    for slot in 0..total_slots {
        let col = slot % 7;
        let day_num = slot - i32::try_from(first_dow).unwrap_or(0) + 1;

        if day_num < 1 || day_num > i32::try_from(num_days).unwrap_or(31) {
            day_spans.push(Span::raw(format!("{:<CELL$}", "")));
            stat_spans.push(Span::raw(format!("{:<CELL$}", "")));
            wc_spans.push(Span::raw(format!("{:<CELL$}", "")));
        } else {
            let d = u32::try_from(day_num).unwrap_or(1);
            let date_key = format!("{year:04}-{month:02}-{d:02}");
            let (stat_text, wc_text) =
                if let Some(&(count, avg, words, chars)) = app.calendar_stats.get(&date_key) {
                    (
                        format!("{count}s {avg:.0}wpm"),
                        format!("{words}w {chars}c"),
                    )
                } else {
                    (String::new(), String::new())
                };
            day_spans.push(Span::styled(
                format!("{d:<CELL$}"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
            stat_spans.push(Span::styled(
                format!("{stat_text:<CELL$}"),
                Style::default().fg(Color::Cyan),
            ));
            wc_spans.push(Span::styled(
                format!("{wc_text:<CELL$}"),
                Style::default().fg(Color::Yellow),
            ));
        }

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
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    all_lines.extend(lines);

    frame.render_widget(Paragraph::new(all_lines), area);
}

fn wiki_row(label: &str, value: String, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {label:<22}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Statistics block for a non-empty paragraph collection.
fn wiki_stats_lines(paragraphs: &[String]) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let total = paragraphs.len();
    lines.push(wiki_row(
        "total paragraphs",
        total.to_string(),
        Color::Yellow,
    ));

    let total_chars: usize = paragraphs.iter().map(std::string::String::len).sum();
    let total_words: usize = paragraphs
        .iter()
        .map(|p| p.split_whitespace().count())
        .sum();
    let avg_len = usize_to_f64(total_chars) / usize_to_f64(total);
    let min_len = paragraphs
        .iter()
        .map(std::string::String::len)
        .min()
        .unwrap_or(0);
    let max_len = paragraphs
        .iter()
        .map(std::string::String::len)
        .max()
        .unwrap_or(0);

    lines.push(wiki_row(
        "total characters",
        total_chars.to_string(),
        Color::Yellow,
    ));
    lines.push(wiki_row(
        "total words",
        total_words.to_string(),
        Color::Yellow,
    ));
    lines.push(wiki_row(
        "avg length",
        format!("{avg_len:.0} chars"),
        Color::Yellow,
    ));
    lines.push(wiki_row(
        "shortest",
        format!("{min_len} chars"),
        Color::Yellow,
    ));
    lines.push(wiki_row(
        "longest",
        format!("{max_len} chars"),
        Color::Yellow,
    ));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Usable paragraphs by length",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    let lengths = [
        TextLength::OneLine,
        TextLength::ShortParagraph,
        TextLength::Paragraph,
        TextLength::LongParagraph,
    ];
    for len in &lengths {
        let min = len.min_chars();
        let max = len.max_chars();
        let count = paragraphs
            .iter()
            .filter(|p| {
                let plen = p.len();
                if plen >= min && plen <= max {
                    return true;
                }
                if plen > max {
                    let trimmed: String = p.chars().take(max).collect();
                    if let Some(pos) = trimmed.rfind(['.', '?', '!']) {
                        return trimmed[..=pos].trim().len() >= min;
                    }
                }
                false
            })
            .count();
        let color = if count > 0 { Color::Green } else { Color::Red };
        lines.push(wiki_row(len.label(), format!("{count}"), color));
    }

    let path = paragraphs_path();
    if let Ok(meta) = std::fs::metadata(&path) {
        let size_kb = u64_to_f64(meta.len()) / 1024.0;
        lines.push(Line::from(""));
        lines.push(wiki_row(
            "file size",
            format!("{size_kb:.0} KB"),
            Color::DarkGray,
        ));
    }
    lines
}

fn render_wikipedia(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Wikipedia collection",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    let paragraphs = load_paragraphs();
    let total = paragraphs.len();

    if total == 0 {
        lines.push(Line::from(Span::styled(
            "  No paragraphs collected yet.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.extend(wiki_stats_lines(&paragraphs));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    if app.wiki_collecting {
        let pct = if app.wiki_target > 0 {
            (usize_to_f64(app.wiki_collected) / usize_to_f64(app.wiki_target) * 100.0).min(100.0)
        } else {
            0.0
        };
        let bar_width = 30usize;
        let filled = f64_to_usize(pct / 100.0 * usize_to_f64(bar_width));
        let empty = bar_width.saturating_sub(filled);
        let bar = format!("[{}{}] {:.0}%", "█".repeat(filled), "░".repeat(empty), pct);

        lines.push(Line::from(Span::styled(
            format!(
                "  Downloading... request {} — {} / {} paragraphs",
                app.wiki_requests, app.wiki_collected, app.wiki_target
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {bar}"),
            Style::default().fg(Color::Cyan),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Press D or Enter to download 1000 paragraphs from Wikipedia",
            Style::default().fg(Color::White),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn stats_row(label: &str, value: String, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {label:<18}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Left/right hand comparison table.
fn hand_table_lines(left_stats: &HandStats, right_stats: &HandStats) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(format!("  {:<18}", ""), Style::default()),
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
        Span::raw(format!("  {:<18}", "avg response")),
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
        Span::raw(format!("  {:<18}", "keys typed")),
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
        Span::raw(format!("  {:<18}", "errors")),
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
        Span::raw(format!("  {:<18}", "error rate")),
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

/// "Last session" body lines.
fn session_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    if app.typing_state == TypingState::Done {
        let accuracy = app.accuracy();
        let acc_color = if accuracy >= 95.0 {
            Color::Green
        } else if accuracy >= 80.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        lines.push(stats_row(
            "speed",
            format!("{:.1} WPM", app.wpm),
            Color::Yellow,
        ));
        lines.push(stats_row("accuracy", format!("{accuracy:.1}%"), acc_color));
        lines.push(stats_row(
            "errors",
            format!("{}", app.errors),
            if app.errors == 0 {
                Color::Green
            } else {
                Color::Red
            },
        ));
        lines.push(stats_row(
            "characters",
            format!("{}", app.target.len()),
            Color::Yellow,
        ));
        lines.push(stats_row(
            "words",
            format!("{}", app.target.len() / 5),
            Color::Yellow,
        ));

        let (left_stats, right_stats) =
            compute_hand_stats(&app.target, &app.typed, &app.keystrokes);

        if left_stats.total_keys > 0 || right_stats.total_keys > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Hand report",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            lines.extend(hand_table_lines(&left_stats, &right_stats));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  No completed session yet. Finish a typing session to see stats.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines
}

/// "All-time totals" heading and body lines.
fn history_lines() -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let stats = load_history_stats();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  All-time totals",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if stats.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No sessions recorded yet.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let mut total_sessions: usize = 0;
        let mut total_wpm_sum: f64 = 0.0;
        let mut total_words: usize = 0;
        let mut total_chars: usize = 0;
        let days_practiced = stats.len();
        for &(sessions, avg_wpm, words, chars) in stats.values() {
            total_sessions += sessions;
            total_wpm_sum += avg_wpm * usize_to_f64(sessions);
            total_words += words;
            total_chars += chars;
        }
        let overall_avg_wpm = if total_sessions > 0 {
            total_wpm_sum / usize_to_f64(total_sessions)
        } else {
            0.0
        };

        lines.push(stats_row(
            "sessions",
            total_sessions.to_string(),
            Color::Yellow,
        ));
        lines.push(stats_row(
            "days practiced",
            days_practiced.to_string(),
            Color::Yellow,
        ));
        lines.push(stats_row(
            "avg WPM",
            format!("{overall_avg_wpm:.1}"),
            Color::Yellow,
        ));

        let wpms = load_all_wpms();
        if !wpms.is_empty() {
            let min_wpm = wpms.iter().copied().fold(f64::INFINITY, f64::min);
            let max_wpm = wpms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let mean = wpms.iter().sum::<f64>() / usize_to_f64(wpms.len());
            let variance =
                wpms.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / usize_to_f64(wpms.len());
            let std_dev = variance.sqrt();

            lines.push(stats_row("min WPM", format!("{min_wpm:.1}"), Color::Red));
            lines.push(stats_row("max WPM", format!("{max_wpm:.1}"), Color::Green));
            lines.push(stats_row(
                "std deviation",
                format!("{std_dev:.1}"),
                Color::Yellow,
            ));
        }

        lines.push(stats_row(
            "total words",
            total_words.to_string(),
            Color::Yellow,
        ));
        lines.push(stats_row(
            "total chars",
            total_chars.to_string(),
            Color::Yellow,
        ));

        let (left_all, right_all) = load_history_hand_stats();
        if left_all.total_keys > 0 || right_all.total_keys > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  All-time hand report",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            lines.extend(hand_table_lines(&left_all, &right_all));
        }
    }

    lines
}

fn render_stats(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Last session",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    lines.extend(session_lines(app));
    lines.extend(history_lines());

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_exit(frame: &mut ratatui::Frame, area: Rect) {
    let stats = load_history_stats();
    let (year, month) = today_ym();
    let today_day = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let (_, _, d) = crate::utils::days_to_ymd(secs / 86400);
        u32::try_from(d).unwrap_or(1)
    };
    let date_key = format!("{year:04}-{month:02}-{today_day:02}");

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Today's training",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    let row = |label: &str, value: String| -> Line<'static> {
        Line::from(vec![
            Span::styled(
                format!("  {label:<16}"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                value,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };

    if let Some(&(sessions, avg_wpm, words, chars)) = stats.get(&date_key) {
        lines.push(row("sessions", sessions.to_string()));
        lines.push(row("avg WPM", format!("{avg_wpm:.1}")));
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
            Span::styled(
                format!("  {label:<18}"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                value.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ])
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  rstype — terminal typing trainer",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  by Mark Veltzer <mark.veltzer@gmail.com>",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        w("version", env!("CARGO_PKG_VERSION"), Color::Yellow),
        w("git describe", env!("GIT_DESCRIBE"), Color::Yellow),
        w("git branch", env!("GIT_BRANCH"), Color::Yellow),
        w("git sha", env!("GIT_SHA"), Color::Yellow),
        w("dirty", env!("GIT_DIRTY"), Color::Yellow),
        w("built", env!("BUILD_TIMESTAMP"), Color::Yellow),
        w("rustc", env!("RUSTC_SEMVER"), Color::Yellow),
        w("rust edition", env!("RUST_EDITION"), Color::Yellow),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

/// One selectable option line of the config screen.
fn option_line(label: &str, selected: bool, active: bool) -> Line<'static> {
    let prefix = if selected { "▶ " } else { "  " };
    let suffix = if active { "  ✓" } else { "" };
    let style = if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else if active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Line::from(Span::styled(format!("{prefix}{label}{suffix}"), style))
}

fn render_config(frame: &mut ratatui::Frame, area: Rect, app: &App) {
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

    let section_style = |active: bool| -> Style {
        if active {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
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
        lines.push(option_line(mode.label(), selected, active));
    }

    if app.config_section == 0 {
        lines.push(Line::from(""));
        let desc = MODES[app.config_cursor].description();
        for desc_line in desc.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {desc_line}"),
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
        lines.push(option_line(src.label(), selected, active));
    }

    if app.config_section == 1 {
        lines.push(Line::from(""));
        let desc = SOURCES[app.config_source_cursor].description();
        for desc_line in desc.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {desc_line}"),
                Style::default().fg(Color::White),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Text length",
        section_style(app.config_section == 2),
    )));
    lines.push(Line::from(""));

    for (i, len) in LENGTHS.iter().enumerate() {
        let selected = app.config_section == 2 && i == app.config_length_cursor;
        let active = *len == app.config.text_length;
        lines.push(option_line(len.label(), selected, active));
    }

    if app.config_section == 2 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", LENGTHS[app.config_length_cursor].description()),
            Style::default().fg(Color::White),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}
