# Minimum Real Estate Requirements

## Overview

The app checks terminal dimensions at startup and refuses to run if the terminal
is too small to render correctly. The minimums are stored in `~/.config/rstype.toml`
as `min_cols` and `min_rows`.

---

## Per-screen analysis

| Screen | Min width | Min height | Notes |
|--------|-----------|------------|-------|
| Toolbar | any | 1 | Always 1 row |
| Status bar | any | 1 | Always 1 row |
| Train | 47 | 5 | Text (43 chars) + box borders + blank lines |
| Train + progress bar | 47 | 8 | Box + progress + stats rows |
| Results | 52 | 8 | Fixed-size results box |
| Config | 60 | 10 | Fixed-size config box |
| Calendar | 74 | 23 | 7 cells × 10 chars + borders; 6 weeks × 3 rows + headers + borders |

## Overall minimum (most demanding screen: Calendar)

```
min_cols = 76    # 74 (calendar box) + 2 side margin
min_rows = 26    # 23 (calendar box) + 2 (toolbar + statusbar) + 1 top/bottom margin
```

These are the **default values**. They can be changed in `~/.config/rstype.toml`:

```toml
mode = "forward"
min_cols = 76
min_rows = 26
```

---

## Behaviour when terminal is too small

On startup, the app queries the terminal size. If either dimension is below the
configured minimum, it prints an error to stderr and exits with code 1:

```
Error: terminal too small (current: 60×20, required: 76×26)
```

---

## Rationale

- **Config option** rather than hard-coded constant: allows users with small
  terminals to lower the minimum if they accept degraded rendering, and allows
  future screens with different requirements to raise it without code changes.
- **Startup check** rather than per-frame: simpler, no need to handle the app
  being "paused" mid-session because the window was resized.
