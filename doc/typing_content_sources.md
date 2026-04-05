# Typing Content Sources

## Overview

Currently the app has a single hard-coded text (Hamlet's soliloquy). This document
captures ideas for how to make the source of typing material configurable and varied.

---

## Ideas

### Bundled content

1. **Built-in texts** — a curated list of famous passages (Hamlet, Lincoln's Gettysburg
   Address, etc.) the user picks from an in-app menu. No external dependencies.

2. **Lessons by difficulty** — structured progression designed for learning:
   home row only → common bigrams → full keyboard. Good for beginners.

---

### File-based

3. **Load from file** — user specifies a file path in `~/.config/rstype.toml`.
   The app reads it at startup. Works for prose, code, lyrics — anything.
   This is the most flexible low-effort option.

4. **Random line from file** — point the app at a quote file or word list;
   it picks a random line each session for endless variety.

---

### Generated

5. **Random common words** — generate a sequence from the top-N English words.
   Endlessly varied, no files needed.

6. **Code snippets** — bundle real code samples (Rust, Python, shell, etc.)
   as typing material. Good for programmers who want domain-relevant practice.

---

### External / dynamic

7. **Wikipedia intro** — fetch the first paragraph of a random Wikipedia article
   via the public API. Fresh content, no setup required.

8. **RSS / news headlines** — fetch from a configurable feed for daily-fresh material.

9. **stdin / pipe** — read target text from standard input, e.g.
   `echo "type this" | rstype` or `cat chapter1.txt | rstype`.
   Makes the app composable with any Unix tool.

---

## Recommendation

Start with **load from file** (option 3) — one config field unlocks everything:
point it at a quote file, a source file, a lyrics file. Add **built-in texts**
(option 1) as the default fallback so the app works out of the box with no config.
