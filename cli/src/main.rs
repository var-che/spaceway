//! Descord CLI - Interactive command-line client
//!
//! Usage:
//!   descord --account alice.key
//!   descord --account bob.key --relay /ip4/127.0.0.1/tcp/9000

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use descord_core::{Client, ClientConfig};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::path::PathBuf;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = Args::parse();

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

    // Create client
    let data_dir = args.data_dir.unwrap_or_else(|| {
        let mut dir = args.account.clone();
        dir.pop();
        dir.push("data");
        dir
    });

    let config = ClientConfig {
        storage_path: data_dir,
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };

    info!("Creating client with config: {:?}", config);
    let client = Client::new(keypair, config)?;

    // Create command handler
    let mut handler = CommandHandler::new(client, account_mgr.username().to_string());

    // Start client in background
    let client_handle = handler.start_background().await?;

    // Interactive REPL
    let mut rl = DefaultEditor::new()?;
    let history_file = args.account.with_extension("history");
    let _ = rl.load_history(&history_file);

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

    // Stop client
    client_handle.abort();

    Ok(())
}
