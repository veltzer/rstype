mod utils;
mod wikipedia;
mod dict;
mod train;
mod ui;

use std::io;
use std::time::Duration;
use clap::{Parser, CommandFactory};
use clap_complete::{Shell, generate};
use crossterm::ExecutableCommand;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::event::{self, Event};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::utils::{Config, load_config, TypingMode, TextSource, TextLength};
use crate::train::App;
use crate::ui::render;
use crate::wikipedia::{cmd_collect, cmd_wikipedia_stats, cmd_wikipedia_clear, cmd_wikipedia_show};
use crate::dict::{cmd_dict_list, cmd_dict_list_remote, cmd_dict_install, cmd_dict_remove, cmd_dict_show};

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
    /// Manage the local Wikipedia paragraph collection
    Wikipedia {
        #[command(subcommand)]
        action: WikipediaAction,
    },
    /// Manage dictionaries for word salad mode
    Dict {
        #[command(subcommand)]
        action: DictAction,
    },
    /// Show detailed version and build information
    Version,
    /// Generate shell completion scripts
    Complete {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(clap::Subcommand)]
enum WikipediaAction {
    /// Download paragraphs from Wikipedia and add them to the local collection
    Download {
        /// Number of total paragraphs to have after downloading
        #[arg(short, long, default_value_t = 1000)]
        count: usize,
    },
    /// Show statistics about the local Wikipedia paragraph collection
    Stats,
    /// Delete the local Wikipedia paragraph collection
    Clear,
    /// Show the file path where Wikipedia paragraphs are stored
    Show,
}

#[derive(clap::Subcommand)]
enum DictAction {
    /// List installed dictionaries
    List,
    /// List dictionaries available for download
    ListRemote,
    /// Install a dictionary (e.g., en-US, de-DE) from wooorm/dictionaries
    Install {
        /// Language code to install (e.g., en-US, de-DE, fr)
        lang: String,
    },
    /// Remove an installed dictionary
    Remove {
        /// Language code to remove
        lang: String,
    },
    /// Show the path to the dictionaries directory
    Show,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Wikipedia { action } => {
            match action {
                WikipediaAction::Download { count } => cmd_collect(count),
                WikipediaAction::Stats => cmd_wikipedia_stats(),
                WikipediaAction::Clear => cmd_wikipedia_clear(),
                WikipediaAction::Show => cmd_wikipedia_show(),
            }
            Ok(())
        }
        Commands::Dict { action } => {
            match action {
                DictAction::List => cmd_dict_list(),
                DictAction::ListRemote => cmd_dict_list_remote(),
                DictAction::Install { lang } => cmd_dict_install(&lang),
                DictAction::Remove { lang } => cmd_dict_remove(&lang),
                DictAction::Show => cmd_dict_show(),
            }
            Ok(())
        }
        Commands::Version => {
            println!("rstype {} by {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"));
            println!("GIT_DESCRIBE: {}", env!("GIT_DESCRIBE"));
            println!("GIT_SHA: {}", env!("GIT_SHA"));
            println!("GIT_BRANCH: {}", env!("GIT_BRANCH"));
            println!("GIT_DIRTY: {}", env!("GIT_DIRTY"));
            println!("RUSTC_SEMVER: {}", env!("RUSTC_SEMVER"));
            println!("RUST_EDITION: {}", env!("RUST_EDITION"));
            println!("BUILD_TIMESTAMP: {}", env!("BUILD_TIMESTAMP"));
            Ok(())
        }
        Commands::Complete { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "rstype", &mut io::stdout());
            Ok(())
        }
        Commands::Train { mode, source, length, min_cols, min_rows } => {
            let mut config = load_config();

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
                app.error_flash = false;
                app.last_pressed_key = None;
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
