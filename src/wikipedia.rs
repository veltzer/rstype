use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::Duration;
use crate::utils::{paragraphs_path, TextLength};

pub enum WikiCollectMsg {
    Progress(usize, u32),
    Done(usize),
}

pub fn load_paragraphs() -> Vec<String> {
    let path = paragraphs_path();
    let Ok(content) = fs::read_to_string(&path) else { return Vec::new(); };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|val| val.get("text").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect()
}

pub fn pick_collected_paragraph(length: TextLength) -> Option<String> {
    let paragraphs = load_paragraphs();
    if paragraphs.is_empty() { return None; }
    let min = length.min_chars();
    let max = length.max_chars();

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

    let candidates: Vec<&String> = paragraphs
        .iter()
        .filter(|p| p.len() >= min && p.len() <= max)
        .collect();

    if candidates.is_empty() {
        let trimmable: Vec<String> = paragraphs
            .iter()
            .filter(|p| p.len() >= min)
            .filter_map(|p| {
                let trimmed: String = p.chars().take(max).collect();
                if let Some(pos) = trimmed.rfind(|c: char| c == '.' || c == '?' || c == '!') {
                    let snapped = trimmed[..=pos].trim().to_string();
                    if snapped.len() >= min { Some(snapped) } else { None }
                } else {
                    None
                }
            })
            .collect();
        if trimmable.is_empty() { return None; }
        let idx = (rng() as usize) % trimmable.len();
        return Some(trimmable[idx].clone());
    }

    let idx = (rng() as usize) % candidates.len();
    Some(candidates[idx].clone())
}

pub fn fetch_wikipedia_paragraphs_batch() -> Vec<String> {
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
        let extract = page.get("extract").and_then(|v| v.as_str()).unwrap_or("");
        for para in extract.split('\n') {
            let trimmed = para.trim();
            if trimmed.len() < 30 { continue; }
            if trimmed.chars().all(|c| c.is_ascii() && c >= ' ' && c != '\x7f') {
                results.push(trimmed.to_string());
            }
        }
    }
    results
}

pub fn cmd_collect(target_count: usize) {
    let path = paragraphs_path();
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }

    let existing = load_paragraphs();
    let mut seen: std::collections::HashSet<String> = existing.into_iter().collect();
    let initial = seen.len();

    eprintln!("Collecting paragraphs from Wikipedia (target: {})...", target_count);
    if initial > 0 { eprintln!("  {} paragraphs already collected", initial); }

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
            if seen.contains(&para) { continue; }
            seen.insert(para.clone());
            if let Ok(json) = serde_json::to_string(&serde_json::json!({ "text": para })) {
                let _ = writeln!(file, "{}", json);
                total += 1;
                added += 1;
            }
            if total >= target_count { break; }
        }
        eprint!("\r  request {} — {} paragraphs collected ({} new this batch)   ", requests, total, added);
        std::thread::sleep(Duration::from_millis(100));
    }
    eprintln!();
    eprintln!("Done! {} paragraphs stored in {}", total, path.display());
}

pub fn cmd_wikipedia_stats() {
    let paragraphs = load_paragraphs();
    let path = paragraphs_path();

    if paragraphs.is_empty() {
        eprintln!("No paragraphs collected yet.");
        eprintln!("Run `rstype wikipedia download` to download paragraphs from Wikipedia.");
        return;
    }

    let total = paragraphs.len();
    let total_chars: usize = paragraphs.iter().map(|p| p.len()).sum();
    let total_words: usize = paragraphs.iter().map(|p| p.split_whitespace().count()).sum();
    let avg_len = total_chars as f64 / total as f64;
    let min_len = paragraphs.iter().map(|p| p.len()).min().unwrap_or(0);
    let max_len = paragraphs.iter().map(|p| p.len()).max().unwrap_or(0);

    println!("Wikipedia collection");
    println!("  file:             {}", path.display());
    if let Ok(meta) = fs::metadata(&path) { println!("  file size:        {:.0} KB", meta.len() as f64 / 1024.0); }
    println!();
    println!("  total paragraphs: {}", total);
    println!("  total characters: {}", total_chars);
    println!("  total words:      {}", total_words);
    println!("  avg length:       {:.0} chars", avg_len);
    println!("  shortest:         {} chars", min_len);
    println!("  longest:          {} chars", max_len);

    println!();
    println!("Usable paragraphs by length:");
    let lengths = [TextLength::OneLine, TextLength::ShortParagraph, TextLength::Paragraph, TextLength::LongParagraph];
    for len in &lengths {
        let min = len.min_chars();
        let max = len.max_chars();
        let count = paragraphs.iter().filter(|p| {
            let plen = p.len();
            if plen >= min && plen <= max { return true; }
            if plen > max {
                let trimmed: String = p.chars().take(max).collect();
                if let Some(pos) = trimmed.rfind(|c: char| c == '.' || c == '?' || c == '!') {
                    return trimmed[..=pos].trim().len() >= min;
                }
            }
            false
        }).count();
        println!("  {:<18} {}", len.label(), count);
    }
}

pub fn cmd_wikipedia_clear() {
    let path = paragraphs_path();
    if path.exists() {
        match fs::remove_file(&path) {
            Ok(()) => eprintln!("Deleted {}", path.display()),
            Err(e) => eprintln!("Error deleting {}: {}", path.display(), e),
        }
    } else {
        eprintln!("Nothing to delete — no collection found at {}", path.display());
    }
}

pub fn cmd_wikipedia_show() {
    println!("{}", paragraphs_path().display());
}
