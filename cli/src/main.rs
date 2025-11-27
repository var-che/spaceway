//! Descord CLI - Interactive command-line client
//!
//! Usage:
//!   descord --account alice.key
//!   descord --account bob.key --relay /ip4/127.0.0.1/tcp/9000

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use spaceway_core::{Client, ClientConfig};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::path::PathBuf;
use std::io::{self, BufRead};
use tracing::info;

mod account;
mod commands;
mod ui;

use account::AccountManager;
use commands::CommandHandler;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to account keypair file (will be created if it doesn't exist)
    #[arg(short, long)]
    account: PathBuf,

    /// Relay node address
    #[arg(short, long, default_value = "/ip4/127.0.0.1/tcp/9000")]
    relay: String,

    /// Data directory
    #[arg(short, long)]
    data_dir: Option<PathBuf>,

    /// Port to listen on (e.g., 0 for random, 9001 for specific port)
    #[arg(short = 'p', long)]
    port: Option<u16>,

    /// Bootstrap peer multiaddr to connect to (e.g., /ip4/127.0.0.1/tcp/9001/p2p/12D3...)
    #[arg(short = 'b', long)]
    bootstrap: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let args = Args::parse();

    // Print banner with version
    println!("{}", "=".repeat(60).bright_blue());
    println!("{}", format!("  {}", spaceway_core::version_string()).bright_cyan().bold());
    println!("{}", "  Privacy-First Decentralized Communication".bright_white());
    println!("{}", "=".repeat(60).bright_blue());
    println!();

    // Load or create account
    let mut account_mgr = AccountManager::new(args.account.clone())?;
    let keypair = account_mgr.load_or_create()?;
    let user_id = keypair.user_id();

    println!("{}", "=".repeat(60).bright_blue());
    println!("{}", "Descord - Privacy-Preserving Decentralized Forum".bright_cyan().bold());
    println!("{}", "=".repeat(60).bright_blue());
    println!();
    println!("{} {}", "Account:".bright_green(), account_mgr.username());
    println!("{} {}", "User ID:".bright_green(), hex::encode(&user_id.as_bytes()[..8]));
    println!("{} {}", "Relay:".bright_green(), args.relay);
    println!();

    // Create client with per-user data directory
    let data_dir = args.data_dir.unwrap_or_else(|| {
        // Use account filename (without .key) as the data dir name
        let account_name = args.account
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("data");
        PathBuf::from(format!("{}-data", account_name))
    });

    let listen_addrs = if let Some(port) = args.port {
        vec![format!("/ip4/0.0.0.0/tcp/{}", port)]
    } else {
        vec![]
    };

    let bootstrap_peers = if let Some(peer) = args.bootstrap {
        vec![peer]
    } else {
        vec![]
    };

    let config = ClientConfig {
        storage_path: data_dir,
        listen_addrs,
        bootstrap_peers,
    };

    info!("Creating client with config: {:?}", config);
    let client = Client::new(keypair, config)?;

    // Create command handler
    let mut handler = CommandHandler::new(client, account_mgr.username().to_string());

    // Start client in background
    let client_handle = handler.start_background().await?;

    // Check if stdin is a terminal or a pipe
    let is_terminal = atty::is(atty::Stream::Stdin);

    if is_terminal {
        // Interactive mode with rustyline (history, editing, etc.)
        run_interactive_mode(&mut handler, &mut account_mgr, args.account).await?;
    } else {
        // Piped/non-interactive mode - simple line reading
        println!("{}", "Running in non-interactive mode (piped input)".bright_yellow());
        run_piped_mode(&mut handler).await?;
    }

    // Stop client
    client_handle.abort();

    Ok(())
}

/// Interactive mode using rustyline for better UX
async fn run_interactive_mode(
    handler: &mut CommandHandler,
    account_mgr: &mut AccountManager,
    account_path: PathBuf,
) -> Result<()> {
    // Interactive REPL
    let mut rl = DefaultEditor::new()?;
    let history_file = account_path.with_extension("history");
    let _ = rl.load_history(&history_file);

    println!("{}", "Type 'help' for available commands, 'quit' to exit".bright_yellow());
    println!();

    println!("{}", "Type 'help' for available commands, 'quit' to exit".bright_yellow());
    println!();

    loop {
        let prompt = format!("{}> ", account_mgr.username().bright_cyan());
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;

                match line {
                    "quit" | "exit" => {
                        println!("{}", "Goodbye!".bright_green());
                        break;
                    }
                    "help" => {
                        ui::print_help();
                    }
                    _ => {
                        if let Err(e) = handler.handle_command(line).await {
                            ui::print_error(&format!("{}", e));
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C".yellow());
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Goodbye!".bright_green());
                break;
            }
            Err(err) => {
                ui::print_error(&format!("Error: {}", err));
                break;
            }
        }
    }

    // Save history
    let _ = rl.save_history(&history_file);
    Ok(())
}

/// Non-interactive mode for piped input (e.g., from tests)
async fn run_piped_mode(handler: &mut CommandHandler) -> Result<()> {
    let stdin = io::stdin();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            "quit" | "exit" => {
                println!("{}", "Goodbye!".bright_green());
                break;
            }
            "help" => {
                ui::print_help();
            }
            _ => {
                if let Err(e) = handler.handle_command(line).await {
                    ui::print_error(&format!("{}", e));
                }
            }
        }
    }

    Ok(())
}
