use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
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

const FALLBACK_TEXT: &str = "\
No paragraphs collected yet. Please run: rstype collect";

const TEST_TEXT: &str = "\
The quick brown fox jumps over the lazy dog. \
Pack my box with five dozen liquor jugs.";

// ── Text fetching ─────────────────────────────────────────────────────────────

const COMMON_WORDS: &[&str] = &[
    "able", "about", "above", "accept", "across", "act", "add", "afraid", "after", "again",
    "agree", "air", "all", "almost", "along", "already", "also", "always", "amount", "an",
    "and", "anger", "angry", "animal", "answer", "any", "appear", "apple", "area", "arm",
    "around", "arrive", "art", "as", "ask", "at", "away", "back", "bad", "bag",
    "ball", "bank", "base", "basket", "bath", "be", "bear", "beat", "beautiful", "because",
    "bed", "before", "began", "begin", "behind", "believe", "below", "beside", "best", "better",
    "between", "big", "bird", "bit", "black", "block", "blood", "blue", "board", "boat",
    "body", "bone", "book", "both", "bottom", "box", "boy", "brain", "branch", "bread",
    "break", "breath", "bridge", "bring", "brother", "brown", "build", "built", "burn", "bus",
    "busy", "but", "buy", "by", "call", "calm", "came", "can", "capital", "car",
    "care", "carry", "cat", "catch", "caught", "cause", "center", "chain", "chair", "chance",
    "change", "charge", "cheap", "check", "chief", "child", "choose", "circle", "city", "claim",
    "class", "clean", "clear", "climb", "clock", "close", "cloud", "coast", "cold", "collect",
    "color", "column", "come", "common", "company", "compare", "complete", "concern", "connect", "consider",
    "contain", "continue", "control", "cook", "cool", "copy", "corner", "correct", "cost", "could",
    "count", "country", "course", "cover", "create", "cross", "cry", "cup", "current", "cut",
    "dance", "danger", "dark", "daughter", "day", "dead", "deal", "dear", "death", "decide",
    "deep", "degree", "depend", "design", "desire", "detail", "develop", "device", "did", "die",
    "dinner", "direct", "direction", "discover", "divide", "do", "doctor", "does", "dog", "dollar",
    "done", "door", "down", "draw", "dream", "dress", "drink", "drive", "drop", "dry",
    "during", "dust", "duty", "each", "ear", "early", "earth", "east", "eat", "edge",
    "effect", "effort", "egg", "eight", "either", "else", "end", "enemy", "engine", "enjoy",
    "enough", "enter", "equal", "escape", "even", "evening", "event", "ever", "every", "exact",
    "example", "except", "excite", "exercise", "exist", "expect", "explain", "express", "extend", "extra",
    "eye", "face", "fact", "factor", "fair", "faith", "fall", "family", "famous", "far",
    "farm", "fast", "fat", "father", "fear", "feed", "feel", "feet", "fell", "few",
    "field", "fight", "figure", "fill", "final", "find", "fine", "finger", "finish", "fire",
    "first", "fish", "five", "flat", "floor", "flower", "fly", "follow", "food", "foot",
    "for", "force", "forest", "forget", "form", "former", "forward", "found", "four", "free",
    "fresh", "friend", "from", "front", "fruit", "full", "fun", "game", "garden", "gate",
    "gather", "gave", "general", "get", "gift", "girl", "give", "glad", "glass", "go",
    "goat", "gold", "gone", "good", "got", "grand", "grass", "great", "green", "grew",
    "ground", "group", "grow", "guard", "guess", "guide", "gun", "had", "hair", "half",
    "hall", "hand", "happen", "happy", "hard", "has", "hat", "have", "he", "head",
    "hear", "heart", "heat", "heavy", "height", "held", "help", "her", "here", "hidden",
    "high", "hill", "him", "his", "hit", "hold", "hole", "home", "hope", "horse",
    "hot", "hotel", "house", "how", "human", "hundred", "hunger", "hunt", "hurry", "hurt",
    "husband", "ice", "idea", "if", "image", "important", "in", "inch", "include", "indeed",
    "indicate", "industry", "inform", "insect", "inside", "instead", "interest", "into", "iron", "island",
    "issue", "it", "item", "its", "itself", "join", "joy", "judge", "jump", "just",
    "keep", "key", "kind", "king", "knew", "knock", "know", "lake", "land", "large",
    "last", "late", "laugh", "lay", "lead", "leader", "learn", "least", "leave", "left",
    "leg", "length", "less", "lesson", "let", "letter", "level", "library", "life", "lift",
    "light", "like", "likely", "limit", "line", "lion", "liquid", "list", "listen", "little",
    "live", "locate", "long", "look", "lose", "lot", "love", "low", "luck", "machine",
    "made", "main", "major", "make", "man", "manner", "many", "map", "march", "mark",
    "market", "master", "match", "material", "matter", "may", "me", "mean", "meet", "member",
    "men", "mental", "method", "middle", "might", "mile", "milk", "million", "mind", "mine",
    "minute", "miss", "mistake", "mix", "modern", "moment", "money", "month", "moon", "moral",
    "more", "morning", "most", "mother", "motion", "mountain", "mouth", "move", "much", "music",
    "must", "my", "name", "narrow", "nation", "nature", "near", "neck", "need", "never",
    "new", "news", "next", "night", "nine", "no", "noise", "none", "nor", "normal",
    "north", "nose", "not", "nothing", "notice", "now", "number", "object", "observe", "obtain",
    "occur", "odd", "of", "off", "offer", "office", "official", "often", "oil", "old",
    "on", "once", "one", "only", "open", "operate", "opinion", "or", "orange", "order",
    "origin", "other", "our", "out", "outer", "outside", "over", "own", "oxygen", "page",
    "paint", "pair", "pan", "panel", "paper", "parent", "part", "partner", "party", "pass",
    "passage", "past", "patient", "pattern", "pay", "people", "per", "perform", "perhaps", "period",
    "person", "phrase", "picture", "piece", "place", "plan", "planet", "plant", "plate", "play",
    "please", "pocket", "poem", "point", "police", "poor", "popular", "port", "position", "possible",
    "pour", "power", "prepare", "present", "press", "pretty", "prevent", "price", "primary", "print",
    "private", "prize", "problem", "produce", "profit", "program", "promise", "proper", "protect", "prove",
    "provide", "public", "pull", "pupil", "purchase", "pure", "push", "put", "quality", "quarter",
    "question", "quick", "quiet", "quite", "race", "radio", "rain", "raise", "ran", "range",
    "rather", "reach", "read", "ready", "real", "reason", "receive", "record", "red", "regard",
    "region", "relate", "remain", "remark", "remember", "repeat", "report", "rest", "result", "return",
    "review", "rich", "ride", "right", "ring", "rise", "river", "road", "rock", "roll",
    "room", "round", "row", "run", "sad", "safe", "said", "sail", "salt", "same",
    "sand", "sat", "save", "saw", "say", "scene", "school", "sea", "search", "season",
    "seat", "second", "section", "see", "seem", "select", "self", "sell", "send", "sense",
    "sentence", "separate", "serve", "set", "settle", "seven", "several", "shape", "share", "sharp",
    "she", "shift", "shine", "ship", "shirt", "shoot", "shore", "short", "should", "shoulder",
    "shout", "show", "shut", "side", "sight", "sign", "signal", "silence", "silver", "simple",
    "since", "sing", "sister", "sit", "six", "size", "skill", "sleep", "slip", "slow",
    "small", "smell", "smile", "smoke", "smooth", "snow", "so", "social", "soft", "solid",
    "solve", "some", "son", "song", "soon", "sorry", "sort", "sound", "source", "south",
    "space", "speak", "special", "speech", "speed", "spend", "spoke", "spread", "spring", "square",
    "staff", "stage", "stand", "standard", "star", "start", "station", "status", "stay", "step",
    "still", "stone", "stop", "store", "storm", "story", "straight", "strange", "stream", "street",
    "strength", "strike", "string", "strong", "student", "study", "such", "sudden", "sugar", "summer",
    "sun", "supply", "support", "sure", "surface", "surprise", "sweet", "swim", "symbol", "system",
    "table", "tail", "take", "talk", "tall", "taste", "teach", "tell", "temple", "ten",
    "term", "test", "than", "that", "the", "their", "them", "then", "there", "these",
    "they", "thick", "thin", "thing", "think", "third", "this", "those", "though", "thought",
    "three", "through", "throw", "tie", "time", "tiny", "title", "to", "today", "together",
    "told", "tomorrow", "tonight", "too", "top", "topic", "total", "touch", "toward", "town",
    "trade", "train", "travel", "tree", "trial", "trouble", "truck", "true", "trust", "truth",
    "try", "turn", "twelve", "twist", "two", "type", "uncle", "under", "unit", "until",
    "up", "upon", "upper", "us", "use", "usual", "valley", "value", "various", "very",
    "view", "village", "voice", "volume", "walk", "wall", "want", "war", "warm", "wash",
    "waste", "watch", "water", "way", "we", "wear", "weather", "week", "well", "went",
    "were", "west", "what", "wheel", "when", "where", "whether", "which", "while", "white",
    "who", "whole", "why", "wide", "wife", "wild", "will", "win", "wind", "window",
    "winter", "wise", "wish", "with", "without", "woman", "wonder", "wood", "word", "work",
    "world", "worry", "worth", "would", "write", "wrong", "year", "yes", "yet", "you",
    "young", "your", "zero",
];

/// Generate a random word salad from the common-words list, respecting the
/// requested length range.  Uses a simple xorshift PRNG seeded from the
/// system clock so no extra dependency is needed.
fn generate_word_salad(length: TextLength) -> String {
    let min = length.min_chars();
    let max = length.max_chars();

    // Seed a simple xorshift64 from the clock
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut state: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    if state == 0 { state = 0xDEAD_BEEF; }
    let mut rng = move || -> u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut result = String::new();
    while result.len() < max {
        let idx = (rng() as usize) % COMMON_WORDS.len();
        let word = COMMON_WORDS[idx];
        if result.is_empty() {
            result.push_str(word);
        } else {
            if result.len() + 1 + word.len() > max {
                break;
            }
            result.push(' ');
            result.push_str(word);
        }
    }
    // If we didn't reach the minimum (unlikely), pad with short words
    while result.len() < min {
        let idx = (rng() as usize) % COMMON_WORDS.len();
        let word = COMMON_WORDS[idx];
        if result.len() + 1 + word.len() <= max {
            result.push(' ');
            result.push_str(word);
        } else {
            break;
        }
    }
    result
}

fn fetch_text(source: TextSource, length: TextLength) -> String {
    match source {
        TextSource::Wikipedia => {
            pick_collected_paragraph(length).unwrap_or_else(|| FALLBACK_TEXT.to_string())
        }
        TextSource::WordSalad => generate_word_salad(length),
    }
}

#[derive(Serialize, Deserialize)]
struct Keystroke {
    typed: String,
    offset_ms: u64,
}

#[derive(Serialize, Deserialize)]
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

fn paragraphs_path() -> PathBuf {
    let mut p = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    p.push(".local");
    p.push("share");
    p.push("rstype");
    p.push("paragraphs.jsonl");
    p
}

/// Load all collected paragraphs from disk.
fn load_paragraphs() -> Vec<String> {
    let path = paragraphs_path();
    let Ok(content) = fs::read_to_string(&path) else { return Vec::new(); };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|val| val.get("text").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect()
}

/// Pick a random paragraph from the local collection that fits the length,
/// falling back to a live Wikipedia fetch if none available.
fn pick_collected_paragraph(length: TextLength) -> Option<String> {
    let paragraphs = load_paragraphs();
    if paragraphs.is_empty() {
        return None;
    }
    let min = length.min_chars();
    let max = length.max_chars();

    // Seed RNG from clock
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut state: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    if state == 0 { state = 0xDEAD_BEEF; }
    let mut rng = move || -> u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    // Collect candidates that fit the length range
    let candidates: Vec<&String> = paragraphs
        .iter()
        .filter(|p| p.len() >= min && p.len() <= max)
        .collect();

    if candidates.is_empty() {
        // Try trimming longer paragraphs to fit
        let trimmable: Vec<String> = paragraphs
            .iter()
            .filter(|p| p.len() >= min)
            .filter_map(|p| {
                let trimmed: String = p.chars().take(max).collect();
                // Snap to last sentence boundary
                if let Some(pos) = trimmed.rfind(|c: char| c == '.' || c == '?' || c == '!') {
                    let snapped = trimmed[..=pos].trim().to_string();
                    if snapped.len() >= min { Some(snapped) } else { None }
                } else {
                    None
                }
            })
            .collect();
        if trimmable.is_empty() {
            return None;
        }
        let idx = (rng() as usize) % trimmable.len();
        return Some(trimmable[idx].clone());
    }

    let idx = (rng() as usize) % candidates.len();
    Some(candidates[idx].clone())
}

/// Fetch all valid ASCII paragraphs from a batch of random Wikipedia articles.
/// Returns a vector of paragraphs (not filtered by length).
fn fetch_wikipedia_paragraphs_batch() -> Vec<String> {
    let resp = ureq::get("https://en.wikipedia.org/w/api.php")
        .query("action", "query")
        .query("generator", "random")
        .query("grnnamespace", "0")
        .query("grnlimit", "20")
        .query("prop", "extracts")
        .query("exintro", "true")
        .query("explaintext", "true")
        .query("format", "json")
        .set("User-Agent", "rstype/1.0 (typing trainer)")
        .call();
    let Ok(resp) = resp else { return Vec::new(); };
    let Ok(json) = resp.into_json::<serde_json::Value>() else { return Vec::new(); };
    let Some(pages) = json.get("query").and_then(|q| q.get("pages")).and_then(|p| p.as_object()) else {
        return Vec::new();
    };

    let mut results = Vec::new();
    for page in pages.values() {
        let extract = page
            .get("extract")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        for para in extract.split('\n') {
            let trimmed = para.trim();
            if trimmed.len() < 30 {
                continue;
            }
            if trimmed
                .chars()
                .all(|c| c.is_ascii() && c >= ' ' && c != '\x7f')
            {
                results.push(trimmed.to_string());
            }
        }
    }
    results
}

/// `rstype collect` — fetch paragraphs from Wikipedia and store locally.
fn cmd_collect(target_count: usize) {
    let path = paragraphs_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Load existing paragraphs to avoid duplicates
    let existing = load_paragraphs();
    let mut seen: std::collections::HashSet<String> = existing.into_iter().collect();
    let initial = seen.len();

    eprintln!(
        "Collecting paragraphs from Wikipedia (target: {})...",
        target_count
    );
    if initial > 0 {
        eprintln!("  {} paragraphs already collected", initial);
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .expect("Failed to open paragraphs file");

    let mut total = initial;
    let mut requests = 0u32;
    while total < target_count {
        requests += 1;
        let batch = fetch_wikipedia_paragraphs_batch();
        let mut added = 0usize;
        for para in batch {
            if seen.contains(&para) {
                continue;
            }
            seen.insert(para.clone());
            if let Ok(json) = serde_json::to_string(&serde_json::json!({ "text": para })) {
                let _ = writeln!(file, "{}", json);
                total += 1;
                added += 1;
            }
            if total >= target_count {
                break;
            }
        }
        eprint!(
            "\r  request {} — {} paragraphs collected ({} new this batch)   ",
            requests, total, added
        );

        // Small delay to be polite to Wikipedia's API
        std::thread::sleep(Duration::from_millis(100));
    }
    eprintln!();
    eprintln!(
        "Done! {} paragraphs stored in {}",
        total,
        path.display()
    );
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

// ── Hand classification (standard QWERTY) ─────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hand {
    Left,
    Right,
}

/// Classify a character as left- or right-hand on a standard QWERTY keyboard.
/// Returns `None` for space (thumb, ambiguous) or non-typeable characters.
fn hand_for_char(c: char) -> Option<Hand> {
    match c.to_ascii_lowercase() {
        // Left-hand rows
        '`' | '1' | '2' | '3' | '4' | '5' => Some(Hand::Left),
        '~' | '!' | '@' | '#' | '$' | '%' => Some(Hand::Left),
        'q' | 'w' | 'e' | 'r' | 't'       => Some(Hand::Left),
        'a' | 's' | 'd' | 'f' | 'g'       => Some(Hand::Left),
        'z' | 'x' | 'c' | 'v' | 'b'       => Some(Hand::Left),
        // Right-hand rows
        '6' | '7' | '8' | '9' | '0' | '-' | '=' => Some(Hand::Right),
        '^' | '&' | '*' | '(' | ')' | '_' | '+' => Some(Hand::Right),
        'y' | 'u' | 'i' | 'o' | 'p' | '[' | ']' | '\\' => Some(Hand::Right),
        'h' | 'j' | 'k' | 'l' | ';' | '\'' => Some(Hand::Right),
        'n' | 'm' | ',' | '.' | '/'        => Some(Hand::Right),
        '{' | '}' | '|' | ':' | '"' | '<' | '>' | '?' => Some(Hand::Right),
        _ => None,
    }
}

/// Per-hand statistics computed from a finished session.
#[derive(Debug, Default)]
struct HandStats {
    avg_response_ms: f64,
    total_keys: usize,
    errors: usize,
}

impl HandStats {
    fn error_rate(&self) -> f64 {
        if self.total_keys == 0 { 0.0 } else { self.errors as f64 / self.total_keys as f64 * 100.0 }
    }
}

/// Compute per-hand response times and error rates from a finished session.
/// Response time for a key = time since previous keystroke.
/// Error rate = fraction of keys for that hand that were typed incorrectly.
fn compute_hand_stats(target: &[char], typed: &[char], keystrokes: &[Keystroke]) -> (HandStats, HandStats) {
    let mut left = HandStats::default();
    let mut right = HandStats::default();
    let mut left_total_ms: f64 = 0.0;
    let mut right_total_ms: f64 = 0.0;

    // Build response times from keystroke offsets (only for Char keystrokes, skip backspace etc.)
    // Pair each accepted character position with its response time.
    let mut char_times: Vec<u64> = Vec::new();
    let mut prev_offset: Option<u64> = None;
    for ks in keystrokes {
        // Only count single-character keys (not "Backspace", "Space" maps to ' ')
        let is_char = ks.typed.len() == 1 || ks.typed == "Space";
        if is_char {
            if let Some(prev) = prev_offset {
                char_times.push(ks.offset_ms.saturating_sub(prev));
            } else {
                // First keystroke — no interval to measure
                char_times.push(0);
            }
        }
        prev_offset = Some(ks.offset_ms);
    }

    // Walk through target positions and match with char_times
    let len = target.len().min(typed.len()).min(char_times.len());
    for i in 0..len {
        let expected = target[i];
        let actual = typed[i];
        let hand = match hand_for_char(expected) {
            Some(h) => h,
            None => continue, // skip space / unknown
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
        // Skip the first character's "interval" (it's 0 / meaningless)
        if i > 0 {
            *total_ms += char_times[i] as f64;
        }
    }

    // Average response times (exclude first key of each hand since it has no interval)
    let left_interval_count = if left.total_keys > 1 { left.total_keys - 1 } else { 0 };
    let right_interval_count = if right.total_keys > 1 { right.total_keys - 1 } else { 0 };
    left.avg_response_ms = if left_interval_count > 0 { left_total_ms / left_interval_count as f64 } else { 0.0 };
    right.avg_response_ms = if right_interval_count > 0 { right_total_ms / right_interval_count as f64 } else { 0.0 };

    (left, right)
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

/// Load all individual session WPM values from history.
fn load_all_wpms() -> Vec<f64> {
    let path = history_path();
    let Ok(content) = fs::read_to_string(&path) else { return Vec::new(); };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|val| val.get("wpm").and_then(|v| v.as_f64()))
        .collect()
}

/// Reconstruct the typed characters from a keystroke log.
fn reconstruct_typed(keystrokes: &[Keystroke]) -> Vec<char> {
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

/// Load all sessions from history and compute aggregate per-hand stats.
fn load_history_hand_stats() -> (HandStats, HandStats) {
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
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
            TextSource::WordSalad => "Generate a random sequence of common English words.\nNo internet required.",
        }
    }
}

fn default_text_source() -> TextSource { TextSource::Wikipedia }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
enum TextLength {
    OneLine,
    ShortParagraph,
    Paragraph,
    LongParagraph,
}

impl TextLength {
    fn label(self) -> &'static str {
        match self {
            TextLength::OneLine        => "One line",
            TextLength::ShortParagraph => "Short paragraph",
            TextLength::Paragraph      => "Paragraph",
            TextLength::LongParagraph  => "Long paragraph",
        }
    }

    fn description(self) -> &'static str {
        match self {
            TextLength::OneLine        => "Around 60 characters — a single sentence.",
            TextLength::ShortParagraph => "Around 150 characters — two or three sentences.",
            TextLength::Paragraph      => "Around 300 characters — a full paragraph.",
            TextLength::LongParagraph  => "Around 600 characters — an extended passage.",
        }
    }

    fn max_chars(self) -> usize {
        match self {
            TextLength::OneLine        => 70,
            TextLength::ShortParagraph => 160,
            TextLength::Paragraph      => 320,
            TextLength::LongParagraph  => 640,
        }
    }

    fn min_chars(self) -> usize {
        match self {
            TextLength::OneLine        => 30,
            TextLength::ShortParagraph => 80,
            TextLength::Paragraph      => 160,
            TextLength::LongParagraph  => 320,
        }
    }
}

fn default_text_length() -> TextLength { TextLength::Paragraph }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    mode: TypingMode,
    #[serde(default = "default_text_source")]
    text_source: TextSource,
    #[serde(default = "default_text_length")]
    text_length: TextLength,
    #[serde(default = "default_min_cols")]
    min_cols: u16,
    #[serde(default = "default_min_rows")]
    min_rows: u16,
}

fn default_min_cols() -> u16 { 76 }
fn default_min_rows() -> u16 { 32 }

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
    Stats,
    Wikipedia,
    Calendar,
    About,
    Exit,
}

/// Messages sent from the background Wikipedia collection thread.
enum WikiCollectMsg {
    /// A batch completed: (total_so_far, request_number)
    Progress(usize, u32),
    /// Collection finished: total paragraphs stored
    Done(usize),
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
    /// Last character key pressed (for keyboard highlight), cleared after render.
    last_pressed_key: Option<char>,
    /// Whether the last pressed key was correct.
    last_pressed_correct: bool,

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
    /// Selected index within the text length list.
    config_length_cursor: usize,
    // calendar
    calendar_year: i32,
    calendar_month: u32,
    calendar_stats: HashMap<String, (usize, f64, usize, usize)>,

    // wikipedia collection
    wiki_collecting: bool,
    wiki_collect_rx: Option<std::sync::mpsc::Receiver<WikiCollectMsg>>,
    wiki_collected: usize,
    wiki_target: usize,
    wiki_requests: u32,
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

    /// Kick off a fresh background text fetch without resetting the whole app.
    fn fetch_new_text(&mut self) {
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

    fn poll_wiki_collect(&mut self) {
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

    fn start_wiki_collect(&mut self, target: usize) {
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
                KeyCode::Char('s') => {
                    self.screen = Screen::Stats;
                    return false;
                }
                KeyCode::Char('w') => {
                    self.screen = Screen::Wikipedia;
                    return false;
                }
                KeyCode::Char('n') => {
                    // Fetch new text — only when not mid-session and not already fetching
                    if !self.fetching && self.typing_state != TypingState::Typing {
                        self.screen = Screen::Typing;
                        self.fetch_new_text();
                    }
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

        // Esc goes back to train screen (from any non-typing screen), or quits if on train
        if key.code == KeyCode::Esc {
            match self.screen {
                Screen::Config | Screen::Stats | Screen::Wikipedia | Screen::Calendar | Screen::About => {
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
            Screen::Config    => self.on_key_config(key),
            Screen::Typing    => self.on_key_typing(key),
            Screen::Calendar  => self.on_key_calendar(key),
            Screen::Wikipedia => self.on_key_wikipedia(key),
            Screen::Stats     => {}
            Screen::About     => {}
            Screen::Exit      => {}
        }
        false
    }

    fn on_key_config(&mut self, key: KeyEvent) {
        const MODES: [TypingMode; 5] = [TypingMode::Forward, TypingMode::Stop, TypingMode::Correct, TypingMode::SuddenDeath, TypingMode::Blind];
        const SOURCES: [TextSource; 2] = [TextSource::Wikipedia, TextSource::WordSalad];
        const LENGTHS: [TextLength; 4] = [TextLength::OneLine, TextLength::ShortParagraph, TextLength::Paragraph, TextLength::LongParagraph];
        match key.code {
            KeyCode::Tab => {
                self.config_section = (self.config_section + 1) % 3;
            }
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

    fn on_key_wikipedia(&mut self, key: KeyEvent) {
        if self.wiki_collecting { return; }
        match key.code {
            KeyCode::Char('d') | KeyCode::Enter => {
                self.start_wiki_collect(1000);
            }
            _ => {}
        }
    }

    fn on_key_typing(&mut self, key: KeyEvent) {
        match self.typing_state {
            TypingState::Done => {
                match key.code {
                    KeyCode::Char('n') => self.fetch_new_text(),
                    KeyCode::Char('r') | KeyCode::Enter | KeyCode::Char(' ') => self.restart(),
                    _ => {}
                }
            }
            TypingState::Waiting | TypingState::Typing => {
                // n fetches a new text (only while waiting, not mid-session)
                if self.typing_state == TypingState::Waiting {
                    if key.code == KeyCode::Char('n') {
                        self.fetch_new_text();
                        return;
                    }
                    // Start timer on first keypress
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
        self.config_length_cursor = match self.config.text_length {
            TextLength::OneLine        => 0,
            TextLength::ShortParagraph => 1,
            TextLength::Paragraph      => 2,
            TextLength::LongParagraph  => 3,
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
            Screen::Config    => render_config(frame, body_rect, app),
            Screen::Stats     => render_stats(frame, indented, app),
            Screen::Wikipedia => render_wikipedia(frame, indented, app),
            Screen::Calendar  => render_calendar(frame, indented, app),
            Screen::About     => render_about(frame, indented),
            Screen::Exit      => render_exit(frame, indented),
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
    let is_stats    = app.screen == Screen::Stats;
    let is_wiki     = app.screen == Screen::Wikipedia;
    let is_calendar = app.screen == Screen::Calendar;
    let is_about    = app.screen == Screen::About;
    let is_exit     = app.screen == Screen::Exit;

    let mut spans = vec![Span::styled("  ", normal_style)];

    for (before, key, after, active) in [
        ("",      'T', "rain",      is_train),
        ("Confi", 'G', "",          is_config),
        ("",      'S', "tats",      is_stats),
        ("",      'W', "ikipedia",  is_wiki),
        ("",      'H', "istory",    is_calendar),
        ("",      'A', "bout",      is_about),
        ("",      'E', "xit",       is_exit),
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
    let left_width: u16 = 60;
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

    // Reserve rows: progress bar + stats + keyboard + hint
    let keyboard_h = 5u16; // 4 key rows + space bar
    let reserved = 5u16 + keyboard_h + 1; // progress+blank+stats+blank + keyboard + hint
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

    // ── On-screen keyboard ────────────────────────────────────────────────
    let kb_y = bar_y + 4;
    let kb_rect = Rect::new(area.x, kb_y, area.width, keyboard_h);
    if kb_rect.bottom() <= area.bottom() {
        render_keyboard(frame, kb_rect, app);
    }

    // ── Ctrl+N hint (shown always except while fetching) ──────────────────
    let mid_session = app.typing_state == TypingState::Typing;
    let key_style = if mid_session {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    };
    let hint = Line::from(vec![
        Span::styled("  ^", Style::default().fg(Color::DarkGray)),
        Span::styled("N", key_style),
        Span::styled(" new text", Style::default().fg(Color::DarkGray)),
    ]);
    let hint_y = area.bottom().saturating_sub(1);
    frame.render_widget(Paragraph::new(hint), Rect::new(area.x, hint_y, area.width, 1));
}

// ── On-screen keyboard ───────────────────────────────────────────────────────

/// Map a character to the unshifted base key on a QWERTY keyboard.
fn base_key(c: char) -> char {
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

fn render_keyboard(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    const ROWS: &[&[char]] = &[
        &['`','1','2','3','4','5','6','7','8','9','0','-','='],
        &['q','w','e','r','t','y','u','i','o','p','[',']','\\'],
        &['a','s','d','f','g','h','j','k','l',';','\''],
        &['z','x','c','v','b','n','m',',','.','/'],
    ];
    // Stagger offsets (in characters) to mimic physical keyboard
    const OFFSETS: &[u16] = &[0, 1, 2, 3];
    const CELL_W: u16 = 3; // width per key cell

    // Expected next key (mapped to base)
    let expected_base = if app.cursor < app.target.len() {
        Some(base_key(app.target[app.cursor]))
    } else {
        None
    };

    // Last pressed key (mapped to base)
    let pressed_base = app.last_pressed_key.map(base_key);

    let dim_style   = Style::default().fg(Color::DarkGray);
    let expect_style = Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD);
    let correct_style = Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD);
    let wrong_style  = Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD);

    for (row_idx, keys) in ROWS.iter().enumerate() {
        if row_idx as u16 >= area.height { break; }
        let y = area.y + row_idx as u16;
        let offset = OFFSETS[row_idx];
        let mut spans: Vec<Span> = Vec::new();
        // Indentation
        if offset > 0 {
            spans.push(Span::raw(" ".repeat(offset as usize)));
        }
        for &k in *keys {
            let display = if k == '\\' { "\\ ".to_string() } else { format!(" {} ", k) };
            let style = if pressed_base == Some(k) {
                if app.last_pressed_correct { correct_style } else { wrong_style }
            } else if expected_base == Some(k) {
                expect_style
            } else {
                dim_style
            };
            spans.push(Span::styled(display, style));
        }
        let line_rect = Rect::new(area.x, y, area.width, 1);
        frame.render_widget(Paragraph::new(Line::from(spans)), line_rect);
    }

    // Space bar row
    let space_row = 4u16;
    if space_row < area.height {
        let y = area.y + space_row;
        let style = if pressed_base == Some(' ') {
            if app.last_pressed_correct { correct_style } else { wrong_style }
        } else if expected_base == Some(' ') {
            expect_style
        } else {
            dim_style
        };
        let mut spans = vec![
            Span::raw(" ".repeat(OFFSETS[3] as usize + CELL_W as usize * 2)),
            Span::styled("   space   ", style),
        ];
        // Show hand-side labels
        spans.push(Span::raw("  "));
        let left_label_style = if app.last_pressed_key.map(|c| hand_for_char(c) == Some(Hand::Left)).unwrap_or(false) {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let right_label_style = if app.last_pressed_key.map(|c| hand_for_char(c) == Some(Hand::Right)).unwrap_or(false) {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled("L", left_label_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("R", right_label_style));
        let line_rect = Rect::new(area.x, y, area.width, 1);
        frame.render_widget(Paragraph::new(Line::from(spans)), line_rect);
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

    let (left_stats, right_stats) = compute_hand_stats(&app.target, &app.typed, &app.keystrokes);

    let mut lines = vec![
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
    ];

    // Hand report
    if left_stats.total_keys > 0 || right_stats.total_keys > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "── Hand Report ──",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Header
        lines.push(Line::from(vec![
            Span::styled(format!("{:<14}", ""), Style::default()),
            Span::styled(format!("{:>10}", "Left"), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:>10}", "Right"), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));

        // Avg response time
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
            Span::styled(format!("{:>10}", left_resp), Style::default().fg(Color::White)),
            Span::styled(format!("{:>10}", right_resp), Style::default().fg(Color::White)),
        ]));

        // Keys typed
        lines.push(Line::from(vec![
            Span::raw(format!("{:<14}", "Keys typed")),
            Span::styled(format!("{:>10}", left_stats.total_keys), Style::default().fg(Color::White)),
            Span::styled(format!("{:>10}", right_stats.total_keys), Style::default().fg(Color::White)),
        ]));

        // Errors
        lines.push(Line::from(vec![
            Span::raw(format!("{:<14}", "Errors")),
            Span::styled(
                format!("{:>10}", left_stats.errors),
                Style::default().fg(if left_stats.errors == 0 { Color::Green } else { Color::Red }),
            ),
            Span::styled(
                format!("{:>10}", right_stats.errors),
                Style::default().fg(if right_stats.errors == 0 { Color::Green } else { Color::Red }),
            ),
        ]));

        // Error rate
        lines.push(Line::from(vec![
            Span::raw(format!("{:<14}", "Error rate")),
            Span::styled(
                format!("{:>9.1}%", left_stats.error_rate()),
                Style::default().fg(if left_stats.error_rate() < 5.0 { Color::Green } else { Color::Red }),
            ),
            Span::styled(
                format!("{:>9.1}%", right_stats.error_rate()),
                Style::default().fg(if right_stats.error_rate() < 5.0 { Color::Green } else { Color::Red }),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Enter / Space / R  retry same text     N  new text",
        Style::default().fg(Color::DarkGray),
    )));

    let box_width = 52u16;
    let result_rect = centered_rect(box_width, lines.len() as u16, area);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        result_rect,
    );
}

fn render_wikipedia(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    let row = |label: &str, value: String, color: Color| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:<22}", label), Style::default().fg(Color::DarkGray)),
            Span::styled(value, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ])
    };

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Wikipedia collection",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
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
        lines.push(row("total paragraphs", total.to_string(), Color::Yellow));

        let total_chars: usize = paragraphs.iter().map(|p| p.len()).sum();
        let total_words: usize = paragraphs.iter().map(|p| p.split_whitespace().count()).sum();
        let avg_len = total_chars as f64 / total as f64;
        let min_len = paragraphs.iter().map(|p| p.len()).min().unwrap_or(0);
        let max_len = paragraphs.iter().map(|p| p.len()).max().unwrap_or(0);

        lines.push(row("total characters", total_chars.to_string(), Color::Yellow));
        lines.push(row("total words", total_words.to_string(), Color::Yellow));
        lines.push(row("avg length", format!("{:.0} chars", avg_len), Color::Yellow));
        lines.push(row("shortest", format!("{} chars", min_len), Color::Yellow));
        lines.push(row("longest", format!("{} chars", max_len), Color::Yellow));

        // Breakdown by length category
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Usable paragraphs by length",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
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
            // Count paragraphs that fit directly or could be trimmed to fit
            let count = paragraphs.iter().filter(|p| {
                let plen = p.len();
                if plen >= min && plen <= max { return true; }
                // Could be trimmed: long enough and has a sentence boundary
                if plen > max {
                    let trimmed: String = p.chars().take(max).collect();
                    if let Some(pos) = trimmed.rfind(|c: char| c == '.' || c == '?' || c == '!') {
                        return trimmed[..=pos].trim().len() >= min;
                    }
                }
                false
            }).count();
            let color = if count > 0 { Color::Green } else { Color::Red };
            lines.push(row(
                len.label(),
                format!("{}", count),
                color,
            ));
        }

        // File path and size
        let path = paragraphs_path();
        if let Ok(meta) = fs::metadata(&path) {
            let size_kb = meta.len() as f64 / 1024.0;
            lines.push(Line::from(""));
            lines.push(row("file size", format!("{:.0} KB", size_kb), Color::DarkGray));
        }
    }

    // Download section
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    if app.wiki_collecting {
        let pct = if app.wiki_target > 0 {
            (app.wiki_collected as f64 / app.wiki_target as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        // Progress bar
        let bar_width = 30usize;
        let filled = (pct / 100.0 * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar = format!("[{}{}] {:.0}%", "█".repeat(filled), "░".repeat(empty), pct);

        lines.push(Line::from(Span::styled(
            format!("  Downloading... request {} — {} / {} paragraphs", app.wiki_requests, app.wiki_collected, app.wiki_target),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("  {}", bar),
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

fn render_stats(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    let row = |label: &str, value: String, color: Color| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("  {:<18}", label), Style::default().fg(Color::DarkGray)),
            Span::styled(value, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ])
    };

    // ── Last session ──────────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Last session",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if app.typing_state == TypingState::Done {
        let accuracy = app.accuracy();
        let acc_color = if accuracy >= 95.0 { Color::Green } else if accuracy >= 80.0 { Color::Yellow } else { Color::Red };

        lines.push(row("speed", format!("{:.1} WPM", app.wpm), Color::Yellow));
        lines.push(row("accuracy", format!("{:.1}%", accuracy), acc_color));
        lines.push(row("errors", format!("{}", app.errors),
            if app.errors == 0 { Color::Green } else { Color::Red }));
        lines.push(row("characters", format!("{}", app.target.len()), Color::Yellow));
        lines.push(row("words", format!("{}", app.target.len() / 5), Color::Yellow));

        // ── Hand report ───────────────────────────────────────────────────
        let (left_stats, right_stats) = compute_hand_stats(&app.target, &app.typed, &app.keystrokes);

        if left_stats.total_keys > 0 || right_stats.total_keys > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Hand report",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            lines.push(Line::from(vec![
                Span::styled(format!("  {:<18}", ""), Style::default()),
                Span::styled(format!("{:>10}", "Left"), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:>10}", "Right"), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]));

            let left_resp = if left_stats.total_keys > 1 {
                format!("{:.0} ms", left_stats.avg_response_ms)
            } else { "—".to_string() };
            let right_resp = if right_stats.total_keys > 1 {
                format!("{:.0} ms", right_stats.avg_response_ms)
            } else { "—".to_string() };
            lines.push(Line::from(vec![
                Span::raw(format!("  {:<18}", "avg response")),
                Span::styled(format!("{:>10}", left_resp), Style::default().fg(Color::White)),
                Span::styled(format!("{:>10}", right_resp), Style::default().fg(Color::White)),
            ]));

            lines.push(Line::from(vec![
                Span::raw(format!("  {:<18}", "keys typed")),
                Span::styled(format!("{:>10}", left_stats.total_keys), Style::default().fg(Color::White)),
                Span::styled(format!("{:>10}", right_stats.total_keys), Style::default().fg(Color::White)),
            ]));

            lines.push(Line::from(vec![
                Span::raw(format!("  {:<18}", "errors")),
                Span::styled(
                    format!("{:>10}", left_stats.errors),
                    Style::default().fg(if left_stats.errors == 0 { Color::Green } else { Color::Red }),
                ),
                Span::styled(
                    format!("{:>10}", right_stats.errors),
                    Style::default().fg(if right_stats.errors == 0 { Color::Green } else { Color::Red }),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw(format!("  {:<18}", "error rate")),
                Span::styled(
                    format!("{:>9.1}%", left_stats.error_rate()),
                    Style::default().fg(if left_stats.error_rate() < 5.0 { Color::Green } else { Color::Red }),
                ),
                Span::styled(
                    format!("{:>9.1}%", right_stats.error_rate()),
                    Style::default().fg(if right_stats.error_rate() < 5.0 { Color::Green } else { Color::Red }),
                ),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  No completed session yet. Finish a typing session to see stats.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // ── All-time totals from history ─────────────────────────────────────
    let stats = load_history_stats();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  All-time totals",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
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
            total_wpm_sum += avg_wpm * sessions as f64;
            total_words += words;
            total_chars += chars;
        }
        let overall_avg_wpm = if total_sessions > 0 { total_wpm_sum / total_sessions as f64 } else { 0.0 };

        lines.push(row("sessions", total_sessions.to_string(), Color::Yellow));
        lines.push(row("days practiced", days_practiced.to_string(), Color::Yellow));
        lines.push(row("avg WPM", format!("{:.1}", overall_avg_wpm), Color::Yellow));

        // Min, max, variance from individual session WPMs
        let wpms = load_all_wpms();
        if !wpms.is_empty() {
            let min_wpm = wpms.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_wpm = wpms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mean = wpms.iter().sum::<f64>() / wpms.len() as f64;
            let variance = wpms.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / wpms.len() as f64;
            let std_dev = variance.sqrt();

            lines.push(row("min WPM", format!("{:.1}", min_wpm), Color::Red));
            lines.push(row("max WPM", format!("{:.1}", max_wpm), Color::Green));
            lines.push(row("std deviation", format!("{:.1}", std_dev), Color::Yellow));
        }

        lines.push(row("total words", total_words.to_string(), Color::Yellow));
        lines.push(row("total chars", total_chars.to_string(), Color::Yellow));

        // ── All-time hand breakdown ───────────────────────────────────
        let (left_all, right_all) = load_history_hand_stats();
        if left_all.total_keys > 0 || right_all.total_keys > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  All-time hand report",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            lines.push(Line::from(vec![
                Span::styled(format!("  {:<18}", ""), Style::default()),
                Span::styled(format!("{:>10}", "Left"), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:>10}", "Right"), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]));

            let left_resp = if left_all.total_keys > 1 {
                format!("{:.0} ms", left_all.avg_response_ms)
            } else { "—".to_string() };
            let right_resp = if right_all.total_keys > 1 {
                format!("{:.0} ms", right_all.avg_response_ms)
            } else { "—".to_string() };
            lines.push(Line::from(vec![
                Span::raw(format!("  {:<18}", "avg response")),
                Span::styled(format!("{:>10}", left_resp), Style::default().fg(Color::White)),
                Span::styled(format!("{:>10}", right_resp), Style::default().fg(Color::White)),
            ]));

            lines.push(Line::from(vec![
                Span::raw(format!("  {:<18}", "keys typed")),
                Span::styled(format!("{:>10}", left_all.total_keys), Style::default().fg(Color::White)),
                Span::styled(format!("{:>10}", right_all.total_keys), Style::default().fg(Color::White)),
            ]));

            lines.push(Line::from(vec![
                Span::raw(format!("  {:<18}", "errors")),
                Span::styled(
                    format!("{:>10}", left_all.errors),
                    Style::default().fg(if left_all.errors == 0 { Color::Green } else { Color::Red }),
                ),
                Span::styled(
                    format!("{:>10}", right_all.errors),
                    Style::default().fg(if right_all.errors == 0 { Color::Green } else { Color::Red }),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::raw(format!("  {:<18}", "error rate")),
                Span::styled(
                    format!("{:>9.1}%", left_all.error_rate()),
                    Style::default().fg(if left_all.error_rate() < 5.0 { Color::Green } else { Color::Red }),
                ),
                Span::styled(
                    format!("{:>9.1}%", right_all.error_rate()),
                    Style::default().fg(if right_all.error_rate() < 5.0 { Color::Green } else { Color::Red }),
                ),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
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
    const LENGTHS: [TextLength; 4] = [TextLength::OneLine, TextLength::ShortParagraph, TextLength::Paragraph, TextLength::LongParagraph];

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

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Text length",
        section_style(app.config_section == 2),
    )));
    lines.push(Line::from(""));

    for (i, len) in LENGTHS.iter().enumerate() {
        let selected = app.config_section == 2 && i == app.config_length_cursor;
        let active = *len == app.config.text_length;
        let prefix = if selected { "▶ " } else { "  " };
        let suffix = if active { "  ✓" } else { "" };
        let label = format!("{}{}{}", prefix, len.label(), suffix);
        let style = if selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(Span::styled(label, style)));
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

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "rstype")]
#[command(about = "Rust based typing trainer")]
#[command(version)]
#[command(subcommand_required = true, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Launch the typing trainer TUI
    Train {
        /// Typing mode (overrides config file)
        #[arg(short, long)]
        mode: Option<TypingMode>,

        /// Text source (overrides config file)
        #[arg(short = 's', long)]
        source: Option<TextSource>,

        /// Text length (overrides config file)
        #[arg(short = 'l', long)]
        length: Option<TextLength>,

        /// Minimum terminal columns
        #[arg(long)]
        min_cols: Option<u16>,

        /// Minimum terminal rows
        #[arg(long)]
        min_rows: Option<u16>,
    },
    /// Collect ~1000 paragraphs from Wikipedia and store them locally for
    /// offline use. Stored in ~/.local/share/rstype/paragraphs.jsonl
    Collect {
        /// Number of paragraphs to collect
        #[arg(short, long, default_value_t = 1000)]
        count: usize,
    },
    /// Generate shell completion scripts
    Complete {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Collect { count } => {
            cmd_collect(count);
            return Ok(());
        }
        Commands::Complete { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "rstype", &mut io::stdout());
            return Ok(());
        }
        Commands::Train { mode, source, length, min_cols, min_rows } => {
            let mut config = load_config();

            // CLI flags override config-file values
            if let Some(mode) = mode { config.mode = mode; }
            if let Some(source) = source { config.text_source = source; }
            if let Some(length) = length { config.text_length = length; }
            if let Some(min_cols) = min_cols { config.min_cols = min_cols; }
            if let Some(min_rows) = min_rows { config.min_rows = min_rows; }

            run_tui(config)
        }
    }
}

fn run_tui(config: Config) -> io::Result<()> {
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
        app.poll_wiki_collect();
        render(&mut terminal, &app)?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if app.on_key(key) {
                    break;
                }
                // Clear flash after it's been rendered once
                app.error_flash = false;
                app.last_pressed_key = None;
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

    // ── Hand classification ───────────────────────────────────────────────

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
        // "ash" => a(left) s(left) h(right)
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
        // target "ash", typed "xsh" — 'x' for 'a' is a left-hand error
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
        // "ah" => a(left, t=0) h(right, t=200)
        let target: Vec<char> = "ah".chars().collect();
        let typed: Vec<char> = "ah".chars().collect();
        let keystrokes = vec![
            Keystroke { typed: "a".into(), offset_ms: 0 },
            Keystroke { typed: "h".into(), offset_ms: 200 },
        ];
        let (left, right) = compute_hand_stats(&target, &typed, &keystrokes);
        // Left has 1 key so no interval avg
        assert_eq!(left.avg_response_ms, 0.0);
        // Right has 1 key so no interval avg either
        assert_eq!(right.avg_response_ms, 0.0);
    }

    #[test]
    fn hand_stats_avg_response_with_enough_keys() {
        // "asdf" => all left: a(t=0) s(t=100) d(t=250) f(t=400)
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
        // intervals: 100, 150, 150 => avg = 400/3 ≈ 133.3
        let expected_avg = (100.0 + 150.0 + 150.0) / 3.0;
        assert!((left.avg_response_ms - expected_avg).abs() < 1.0);
    }

    // ── Word salad ────────────────────────────────────────────────────────

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
