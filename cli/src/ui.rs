//! UI utilities for pretty printing

use colored::Colorize;

pub fn print_help() {
    println!();
    println!("{}", "Available Commands:".bright_cyan().bold());
    println!();
    println!("  {:<30} {}", "help".bright_green(), "Show this help message");
    println!("  {:<30} {}", "quit, exit".bright_green(), "Exit the application");
    println!();
    println!("  {}", "Spaces:".bright_yellow().bold());
    println!("  {:<30} {}", "spaces".bright_green(), "List all spaces");
    println!("  {:<30} {}", "space create <name>".bright_green(), "Create a new space");
    println!("  {:<30} {}", "space <id>".bright_green(), "Switch to a space");
    println!();
    println!("  {}", "Channels:".bright_yellow().bold());
    println!("  {:<30} {}", "channels".bright_green(), "List channels in current space");
    println!("  {:<30} {}", "channel create <name>".bright_green(), "Create a channel");
    println!("  {:<30} {}", "channel <id>".bright_green(), "Switch to a channel");
    println!();
    println!("  {}", "Threads:".bright_yellow().bold());
    println!("  {:<30} {}", "threads".bright_green(), "List threads in current channel");
    println!("  {:<30} {}", "thread create <title>".bright_green(), "Create a thread");
    println!("  {:<30} {}", "thread <id>".bright_green(), "Switch to a thread");
    println!();
    println!("  {}", "Messages:".bright_yellow().bold());
    println!("  {:<30} {}", "messages".bright_green(), "Show messages in current thread");
    println!("  {:<30} {}", "send <text>".bright_green(), "Send a message");
    println!();
    println!("  {}", "Invites:".bright_yellow().bold());
    println!("  {:<30} {}", "invite".bright_green(), "List active invites");
    println!("  {:<30} {}", "invite create".bright_green(), "Create an invite code");
    println!("  {:<30} {}", "join <space_id> <code>".bright_green(), "Join with invite code");
    println!("  {:<30} {}", "join dht <space_id>".bright_green(), "Join from DHT (offline)");
    println!();
    println!("  {}", "Files:".bright_yellow().bold());
    println!("  {:<30} {}", "upload <file>".bright_green(), "Upload file to current space");
    println!();
    println!("  {}", "Info:".bright_yellow().bold());
    println!("  {:<30} {}", "whoami".bright_green(), "Show current user info");
    println!("  {:<30} {}", "context".bright_green(), "Show current context");
    println!("  {:<30} {}", "refresh".bright_green(), "Refresh network status");
    println!();
}

pub fn print_error(msg: &str) {
    println!("{} {}", "✗".bright_red(), msg.red());
}

pub fn print_success(msg: &str) {
    println!("{} {}", "✓".bright_green(), msg.bright_green());
}

pub fn print_info(msg: &str) {
    println!("{} {}", "ℹ".bright_blue(), msg);
}

pub fn print_warning(msg: &str) {
    println!("{} {}", "⚠".bright_yellow(), msg.yellow());
}
