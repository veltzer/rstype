# RSType - Rust Typing Trainer

A fast, terminal-based typing trainer written in Rust with multiple typing modes, Wikipedia content, word salad dictionaries, and session history tracking.

## Features

- **Multiple typing modes** — Forward, Stop, Correct, Sudden Death, and Blind modes for varied training
- **Wikipedia content** — fetch random Wikipedia paragraphs for fresh, varied typing material
- **Word salad mode** — generate practice text from installable dictionaries (en-US, de-DE, fr, etc.)
- **Configurable text length** — one line, short paragraph, paragraph, or long paragraph
- **Session history** — every session is recorded in JSONL format with per-keystroke timing data
- **Calendar view** — browse training history by month in an interactive calendar
- **In-app configuration** — switch modes and settings from the Config screen
- **TUI interface** — clean terminal UI built with Ratatui, with toolbar and status bar
- **Shell completions** — generate completions for Bash, Zsh, Fish, PowerShell, and Elvish
- **Cross-platform** — builds for Linux (x86_64, aarch64), macOS, and Windows

## Quick Start

```bash
rstype wikipedia download            # Download Wikipedia paragraphs
rstype train                         # Launch the typing trainer
rstype train --mode forward          # Train in forward mode
rstype train --source word-salad     # Train with word salad text
rstype version                       # Show version and build info
```

## Typing Modes

| Mode | Description |
|------|-------------|
| **Forward** | Cursor advances even on wrong key; errors shown in red |
| **Stop** | Cursor stays on wrong key until the correct one is pressed |
| **Correct** | Like Forward but must correct all errors before finishing |
| **Sudden Death** | One mistake resets the entire session immediately |
| **Blind** | Typed characters are hidden (shown as ·); no visual feedback |

## Philosophy

Practice-oriented, zero-friction typing training — launch and start typing with minimal setup. Convention over configuration with sensible defaults.
