//! UI utilities for pretty printing

use colored::Colorize;

pub fn print_help() {
    println!();
    println!("{}", "Available Commands:".bright_cyan().bold());
    println!();
    println!("  {:<25} {}", "help".bright_green(), "Show this help message");
    println!("  {:<25} {}", "quit, exit".bright_green(), "Exit the application");
    println!();
    println!("  {}", "Spaces:".bright_yellow().bold());
    println!("  {:<25} {}", "spaces".bright_green(), "List all spaces");
    println!("  {:<25} {}", "space create <name>".bright_green(), "Create a new space");
    println!("  {:<25} {}", "space <id>".bright_green(), "Switch to a space");
    println!();
    println!("  {}", "Channels:".bright_yellow().bold());
    println!("  {:<25} {}", "channels".bright_green(), "List channels in current space");
    println!("  {:<25} {}", "channel create <name>".bright_green(), "Create a channel");
    println!("  {:<25} {}", "channel <id>".bright_green(), "Switch to a channel");
    println!();
    println!("  {}", "Threads:".bright_yellow().bold());
    println!("  {:<25} {}", "threads".bright_green(), "List threads in current channel");
    println!("  {:<25} {}", "thread create <title>".bright_green(), "Create a thread");
    println!("  {:<25} {}", "thread <id>".bright_green(), "Switch to a thread");
    println!();
    println!("  {}", "Messages:".bright_yellow().bold());
    println!("  {:<25} {}", "messages".bright_green(), "Show messages in current thread");
    println!("  {:<25} {}", "send <text>".bright_green(), "Send a message");
    println!();
    println!("  {}", "Info:".bright_yellow().bold());
    println!("  {:<25} {}", "whoami".bright_green(), "Show current user info");
    println!("  {:<25} {}", "context".bright_green(), "Show current context (space/channel/thread)");
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
