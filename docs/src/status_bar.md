# Status Bar Design

## Overview

A single-row status bar is displayed at the bottom of the terminal window on all
screens. It is always visible and provides authorship and branding information.

---

## Location

The status bar occupies the last row of the terminal (`area.height - 1`). The
usable body area (toolbar to status bar) is therefore `area.height - 2` rows.

```
┌─────────────────────────────────────┐  ← row 0: toolbar
│                                     │
│           body content              │
│                                     │
└─────────────────────────────────────┘  ← last row: status bar
```

---

## Content

```
rstype by Mark Veltzer <mark.veltzer@gmail.com>
```

- Left-aligned
- Padded with spaces to fill the full terminal width
- No dynamic content — it never changes at runtime

---

## Visual style

| Property | Value |
|----------|-------|
| Background | `White` |
| Foreground | `Black` |
| Modifier | none |

Matches the toolbar at the top — both are white bars, framing the black body in
between. This gives the UI a consistent, symmetrical appearance.

---

## Rationale

- **Branding** — identifies the application and its author in any screenshot or recording.
- **Symmetry** — the toolbar occupies the top row; a status bar at the bottom gives
  the UI a balanced, framed appearance common in terminal applications (vim, htop, etc.).
- **Static content** — future versions could use the status bar to show transient
  messages (e.g. "Config saved") without disrupting the main body layout, since the
  row is already reserved.
