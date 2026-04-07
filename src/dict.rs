use std::fs;
use std::path::PathBuf;
use crate::utils::{dirs_home, TextLength};

pub const COMMON_WORDS: &[&str] = &[
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

pub fn dict_dir() -> PathBuf {
    let mut p = dirs_home().unwrap_or_else(|| PathBuf::from("."));
    p.push(".local");
    p.push("share");
    p.push("rstype");
    p.push("dicts");
    p
}

/// Load words from an installed dictionary's .dic file.
/// Returns only lowercase ASCII words (2–10 chars) suitable for word salad.
pub fn load_dict_words(lang: &str) -> Vec<String> {
    let dir = dict_dir();
    let dic_path = dir.join(format!("{}.dic", lang));
    let Ok(content) = fs::read_to_string(&dic_path) else { return Vec::new(); };
    content
        .lines()
        .skip(1) // first line is word count
        .map(|line| {
            // .dic lines may have affix flags after '/'
            line.split('/').next().unwrap_or("").trim().to_lowercase()
        })
        .filter(|w| {
            w.len() >= 2
                && w.len() <= 10
                && w.chars().all(|c| c.is_ascii_lowercase())
        })
        .collect()
}

/// Load words from all installed dictionaries, deduplicated.
pub fn load_all_dict_words() -> Vec<String> {
    let dir = dict_dir();
    let Ok(entries) = fs::read_dir(&dir) else { return Vec::new(); };
    let mut seen = std::collections::HashSet::new();
    let mut words = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("dic") {
            if let Some(lang) = path.file_stem().and_then(|s| s.to_str()) {
                for w in load_dict_words(lang) {
                    if seen.insert(w.clone()) {
                        words.push(w);
                    }
                }
            }
        }
    }
    words
}

/// Generate a random word salad from the common-words list, respecting the
/// requested length range.  Uses a simple xorshift PRNG seeded from the
/// system clock so no extra dependency is needed.
pub fn generate_word_salad(length: TextLength) -> String {
    let min = length.min_chars();
    let max = length.max_chars();

    // Use installed dictionary words if available, otherwise fall back to embedded list
    let dict_words = if cfg!(test) { Vec::new() } else { load_all_dict_words() };
    let use_dict = !dict_words.is_empty();

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

    let pick_word = |rng_val: u64| -> &str {
        if use_dict {
            &dict_words[(rng_val as usize) % dict_words.len()]
        } else {
            COMMON_WORDS[(rng_val as usize) % COMMON_WORDS.len()]
        }
    };

    let mut result = String::new();
    while result.len() < max {
        let word = pick_word(rng());
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
        let word = pick_word(rng());
        if result.len() + 1 + word.len() <= max {
            result.push(' ');
            result.push_str(word);
        } else {
            break;
        }
    }
    result
}

pub fn cmd_dict_show() {
    println!("{}", dict_dir().display());
}

pub fn cmd_dict_list() {
    let dir = dict_dir();
    println!("Dictionaries stored in: {}", dir.display());
    println!();

    let Ok(entries) = fs::read_dir(&dir) else {
        println!("  (directory does not exist yet)");
        return;
    };

    let mut langs: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("dic") {
                p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();
    langs.sort();

    if langs.is_empty() {
        println!("  (none installed)");
    } else {
        println!("Installed languages:");
        for lang in &langs {
            let words = load_dict_words(lang);
            println!("  {:<12} {} usable words", lang, words.len());
        }
    }
    println!();
    println!("Embedded fallback: {} words", COMMON_WORDS.len());
}

pub fn cmd_dict_list_remote() {
    let url = "https://api.github.com/repos/wooorm/dictionaries/contents/dictionaries";
    eprintln!("Fetching available dictionaries from wooorm/dictionaries...");

    let resp = ureq::get(url)
        .set("User-Agent", "rstype/1.0")
        .call();
    let Ok(resp) = resp else {
        eprintln!("Error: failed to fetch dictionary list.");
        return;
    };
    let Ok(contents) = resp.into_json::<Vec<serde_json::Value>>() else {
        eprintln!("Error: failed to parse response.");
        return;
    };

    let mut langs: Vec<String> = contents
        .iter()
        .filter(|item| item.get("type").and_then(|v| v.as_str()) == Some("dir"))
        .filter_map(|item| item.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();
    langs.sort();

    println!("Available languages ({}):", langs.len());
    for lang in &langs {
        println!("  {}", lang);
    }
    println!();
    println!("Install with: rstype dict install <lang>");
}

pub fn cmd_dict_install(lang: &str) {
    let dir = dict_dir();
    let _ = fs::create_dir_all(&dir);
    let lang_normalized = lang.replace('_', "-");

    let dic_url = format!(
        "https://raw.githubusercontent.com/wooorm/dictionaries/main/dictionaries/{}/index.dic",
        lang_normalized
    );

    eprintln!("Downloading dictionary for {}...", lang_normalized);

    let resp = ureq::get(&dic_url)
        .set("User-Agent", "rstype/1.0")
        .call();
    let Ok(resp) = resp else {
        eprintln!("Error: failed to download .dic file. Check if '{}' exists at https://github.com/wooorm/dictionaries", lang_normalized);
        return;
    };
    let Ok(content) = resp.into_string() else {
        eprintln!("Error: failed to read response.");
        return;
    };

    let dest = dir.join(format!("{}.dic", lang_normalized));
    if let Err(e) = fs::write(&dest, &content) {
        eprintln!("Error writing {}: {}", dest.display(), e);
        return;
    }

    let words = load_dict_words(&lang_normalized);
    eprintln!("Installed {} ({} usable words for word salad)", lang_normalized, words.len());
}

pub fn cmd_dict_remove(lang: &str) {
    let dir = dict_dir();
    let lang_normalized = lang.replace('_', "-");
    let dic_path = dir.join(format!("{}.dic", lang_normalized));

    if dic_path.exists() {
        match fs::remove_file(&dic_path) {
            Ok(()) => eprintln!("Removed {}", lang_normalized),
            Err(e) => eprintln!("Error removing {}: {}", dic_path.display(), e),
        }
    } else {
        eprintln!("Dictionary '{}' is not installed.", lang_normalized);
    }
}
