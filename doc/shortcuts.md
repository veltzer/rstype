# Keyboard Shortcuts

## Global shortcuts (work on every screen)

| Key | Action |
|-----|--------|
| `Ctrl+C` | Exit the application (intercepted by app to cleanly restore terminal) |
| `Ctrl+T` | Go to Train screen |
| `Ctrl+G` | Go to Config screen |
| `Ctrl+H` | Go to History (calendar) screen |
| `Ctrl+E` | Exit the application cleanly |
| `Esc` | Go back to Train screen (from Config or History), or exit if already on Train |

---

## Train screen

| Key | Action |
|-----|--------|
| Any character key | Start session (on first keypress), type character |
| `Backspace` | Delete last character (move cursor back) |
| `Space` / `Enter` / `R` | Restart session (only when session is Done) |

---

## Config screen

| Key | Action |
|-----|--------|
| `↑` / `↓` | Move selection between modes |
| `Enter` | Save selected mode and return to Train screen |

---

## History (calendar) screen

| Key | Action |
|-----|--------|
| `←` | Go to previous month |
| `→` | Go to next month |

---

## Notes

- `Ctrl+C` is intercepted by the app (raw mode captures it before the OS). It exits
  cleanly, restoring the terminal. This is preferable to a raw SIGINT which would
  leave the terminal in raw mode.
- Navigating away from the Train screen mid-session silently discards the
  in-progress session (it is not saved to history).
