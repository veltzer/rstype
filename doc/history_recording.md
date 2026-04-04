# History Recording Design

## Overview

Every completed training session is appended as a single JSON line to a log file.
This allows post-hoc analysis of typing speed, accuracy trends, and per-key performance
without any in-app reporting UI.

---

## File location

```
~/.local/share/rstype/history.jsonl
```

**Why `~/.local/share/rstype/`?**
This follows the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/)
for user-specific application data on Linux. It keeps training data separate from
configuration (`~/.config/rstype.toml`) and out of the home directory root.

---

## File format: JSONL

One JSON object per line, appended on every session completion.

**Why JSONL over SQLite?**
SQLite would enable richer in-process queries but adds a native dependency and
significantly more complexity. JSONL is append-only, requires no schema migrations,
is readable with any text tool (`cat`, `grep`, `jq`), and can be imported into any
analysis tool (Python, R, DuckDB, etc.) trivially.

**Why JSONL over plain CSV?**
The `keystrokes` field is a variable-length array. CSV cannot represent nested
structures cleanly without quoting hacks or splitting into multiple files.

---

## Record structure

```json
{
  "timestamp": "2026-04-04T15:08:00Z",
  "text": "The quick brown fox jumps over the lazy dog",
  "mode": "forward",
  "wpm": 65.3,
  "errors": 2,
  "keystrokes": [
    {"typed": "T", "offset_ms": 0},
    {"typed": "h", "offset_ms": 312},
    {"typed": "Backspace", "offset_ms": 850},
    {"typed": "h", "offset_ms": 1103}
  ]
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | string | ISO 8601 UTC timestamp of when the session completed |
| `text` | string | The target text that was trained on |
| `mode` | string | Typing mode: `"forward"` or `"stop"` |
| `wpm` | number | Words per minute (chars / 5 / elapsed minutes) |
| `errors` | number | Total wrong keypresses during the session |
| `keystrokes` | array | Ordered list of every key pressed, from first to last |

### Keystroke fields

| Field | Type | Description |
|-------|------|-------------|
| `typed` | string | The key that was pressed, in W3C format (see below) |
| `offset_ms` | number | Milliseconds since the first keypress of the session |

**Why record every keypress (not just correct ones)?**
Wrong keypresses, backspaces, and hesitations are precisely where skill gaps live.
Recording only correct keys would discard the most valuable training signal.

**Why no `expected` field per keystroke?**
It is redundant. The target text is stored in the `text` field, and the position
of each correct keystroke in the sequence can be derived from it. Storing `expected`
would bloat the log for no additional information.

---

## Key name standard: W3C KeyboardEvent `key` values

Key names follow the [W3C UI Events KeyboardEvent key Values](https://www.w3.org/TR/uievents-key/)
specification.

Examples:

| Key | Stored as |
|-----|-----------|
| Letter a | `"a"` |
| Letter A (shifted) | `"A"` |
| Space | `"` `"` (literal space character) |
| Backspace | `"Backspace"` |
| Enter | `"Enter"` |
| CapsLock | `"CapsLock"` |
| Left arrow | `"ArrowLeft"` |
| Escape | `"Escape"` |

**Why W3C key values over USB HID keycodes (numbers)?**
USB HID codes are more compact (1–2 digits vs. 9 characters for `"Backspace"`)
but the size difference is negligible (~2 bytes per keystroke over a 43-character
exercise). W3C key strings are self-documenting, require no lookup table to
interpret, and are the closest thing to a universal cross-platform string standard
for key names.
