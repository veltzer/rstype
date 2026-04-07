# RSType - Rust Typing Trainer

A fast, terminal-based typing trainer written in Rust with multiple typing modes, Wikipedia content, word salad dictionaries, and session history tracking.

## Documentation

Full documentation: https://veltzer.github.io/rstype/

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

## Installation

### Download pre-built binary (Linux)

Pre-built binaries are available for Linux, macOS, and Windows.

```bash
# Linux x86_64
gh release download latest --repo veltzer/rstype --pattern 'rstype-linux-x86_64' --output rstype --clobber

# Linux aarch64 / arm64
gh release download latest --repo veltzer/rstype --pattern 'rstype-linux-aarch64' --output rstype --clobber

chmod +x rstype
sudo mv rstype /usr/local/bin/
```

Or without the GitHub CLI:

```bash
# Linux x86_64
curl -Lo rstype https://github.com/veltzer/rstype/releases/latest/download/rstype-linux-x86_64

# Linux aarch64 / arm64
curl -Lo rstype https://github.com/veltzer/rstype/releases/latest/download/rstype-linux-aarch64

chmod +x rstype
sudo mv rstype /usr/local/bin/
```

### Build from source

```bash
cargo build --release
```

## Quick Start

```bash
rstype wikipedia download            # Download Wikipedia paragraphs
rstype train                         # Launch the typing trainer
rstype train --mode forward          # Train in forward mode
rstype train --source word-salad     # Train with word salad text
rstype train --length one-line       # Train with short texts
rstype wikipedia stats               # Show Wikipedia collection stats
rstype dict list-remote              # List available dictionaries
rstype dict install en-US            # Install English dictionary
rstype version                       # Show version and build info
rstype complete bash                 # Generate shell completions
```

## Typing Modes

| Mode | Description |
|------|-------------|
| **Forward** | Cursor advances even on wrong key; errors shown in red |
| **Stop** | Cursor stays on wrong key until the correct one is pressed |
| **Correct** | Like Forward but must correct all errors before finishing |
| **Sudden Death** | One mistake resets the entire session immediately |
| **Blind** | Typed characters are hidden (shown as ·); no visual feedback |

## Author

Mark Veltzer <mark.veltzer@gmail.com>

## License

MIT
