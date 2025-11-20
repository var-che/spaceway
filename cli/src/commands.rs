//! Command handler for CLI

use anyhow::{Context, Result};
use colored::Colorize;
use descord_core::{Client, SpaceId, ChannelId, ThreadId};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::ui;

pub struct CommandHandler {
    client: Arc<Mutex<Client>>,
    username: String,
    current_space: Option<SpaceId>,
    current_channel: Option<ChannelId>,
    current_thread: Option<ThreadId>,
}

impl CommandHandler {
    pub fn new(client: Client, username: String) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            username,
            current_space: None,
            current_channel: None,
            current_thread: None,
        }
    }

    /// Start client background tasks  
    pub async fn start_background(&self) -> Result<JoinHandle<()>> {
        let client = Arc::clone(&self.client);
        let handle = tokio::spawn(async move {
            let c = client.lock().await;
            if let Err(e) = c.start().await {
                eprintln!("Client error: {}", e);
            }
        });
        
        // Give the client a moment to start up
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        Ok(handle)
    }

    pub async fn handle_command(&mut self, input: &str) -> Result<()> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "whoami" => self.cmd_whoami().await,
            "context" => self.cmd_context(),
            "spaces" => self.cmd_spaces().await,
            "space" => self.cmd_space(&parts[1..]).await,
            "channels" => self.cmd_channels().await,
            "channel" => self.cmd_channel(&parts[1..]).await,
            "threads" => self.cmd_threads().await,
            "thread" => self.cmd_thread(&parts[1..]).await,
            "messages" => self.cmd_messages().await,
            "send" => self.cmd_send(&parts[1..].join(" ")).await,
            "invite" => self.cmd_invite(&parts[1..]).await,
            "join" => self.cmd_join(&parts[1..]).await,
            "upload" => self.cmd_upload(&parts[1..]).await,
            "refresh" => self.cmd_refresh().await,
            _ => {
                ui::print_error(&format!("Unknown command: {}", parts[0]));
                ui::print_info("Type 'help' for available commands");
                Ok(())
            }
        }
    }

    async fn cmd_whoami(&self) -> Result<()> {
        let user_id = {
            let client = self.client.lock().await;
            client.user_id()
        };
        println!();
        println!("{} {}", "Username:".bright_green(), self.username.bright_cyan());
        println!("{} {}", "User ID:".bright_green(), hex::encode(&user_id.as_bytes()[..16]));
        println!();
        Ok(())
    }

    fn cmd_context(&self) -> Result<()> {
        println!();
        println!("{}", "Current Context:".bright_cyan().bold());
        
        if let Some(space_id) = &self.current_space {
            println!("  {}: {}", "Space".bright_green(), hex::encode(space_id.0));
        } else {
            println!("  {}: {}", "Space".bright_green(), "none".yellow());
        }

        if let Some(channel_id) = &self.current_channel {
            println!("  {}: {}", "Channel".bright_green(), hex::encode(channel_id.0));
        } else {
            println!("  {}: {}", "Channel".bright_green(), "none".yellow());
        }

        if let Some(thread_id) = &self.current_thread {
            println!("  {}: {}", "Thread".bright_green(), hex::encode(thread_id.0));
        } else {
            println!("  {}: {}", "Thread".bright_green(), "none".yellow());
        }

        println!();
        Ok(())
    }

    async fn cmd_spaces(&self) -> Result<()> {
        let spaces = {
            let client = self.client.lock().await;
            client.list_spaces().await
        };
        
        println!();
        if spaces.is_empty() {
            ui::print_info("No spaces yet. Create one with: space create <name>");
        } else {
            println!("{} ({}):", "Spaces".bright_cyan().bold(), spaces.len());
            for space in spaces {
                let id_short = hex::encode(&space.id.0[..8]);
                let marker = if Some(space.id) == self.current_space {
                    "→".bright_green()
                } else {
                    " ".normal()
                };
                println!("  {} {} - {}", marker, id_short.bright_yellow(), space.name);
            }
        }
        println!();
        Ok(())
    }

    async fn cmd_space(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            ui::print_error("Usage: space create <name>  OR  space <id>");
            return Ok(());
        }

        if args[0] == "create" {
            let name = args[1..].join(" ");
            if name.is_empty() {
                ui::print_error("Space name cannot be empty");
                return Ok(());
            }

            let (space, _op, _privacy_info) = {
                let client = self.client.lock().await;
                client.create_space(
                    name.clone(),
                    Some(format!("Created by {}", self.username)),
                ).await?
            };

            self.current_space = Some(space.id);
            self.current_channel = None;
            self.current_thread = None;

            ui::print_success(&format!("Created space: {} ({})", name, hex::encode(&space.id.0[..8])));
        } else {
            // Switch to space by ID prefix
            let prefix = args[0];
            let spaces = {
                let client = self.client.lock().await;
                client.list_spaces().await
            };
            
            let matches: Vec<_> = spaces.into_iter()
                .filter(|s| hex::encode(s.id.0).starts_with(prefix))
                .collect();

            match matches.len() {
                0 => ui::print_error(&format!("No space found with ID prefix: {}", prefix)),
                1 => {
                    self.current_space = Some(matches[0].id);
                    self.current_channel = None;
                    self.current_thread = None;
                    ui::print_success(&format!("Switched to space: {}", matches[0].name));
                }
                _ => {
                    ui::print_error("Multiple spaces match that prefix. Be more specific:");
                    for space in matches {
                        println!("  {} - {}", hex::encode(&space.id.0[..8]), space.name);
                    }
                }
            }
        }

        Ok(())
    }

    async fn cmd_channels(&self) -> Result<()> {
        let space_id = self.current_space.context("No space selected. Use: space <id>")?;
        let channels = {
            let client = self.client.lock().await;
            client.list_channels(&space_id).await
        };
        
        println!();
        if channels.is_empty() {
            ui::print_info("No channels yet. Create one with: channel create <name>");
        } else {
            println!("{} ({}):", "Channels".bright_cyan().bold(), channels.len());
            for channel in channels {
                let id_short = hex::encode(&channel.id.0[..8]);
                let marker = if Some(channel.id) == self.current_channel {
                    "→".bright_green()
                } else {
                    " ".normal()
                };
                let status = if channel.archived { " [archived]".red() } else { "".normal() };
                println!("  {} {} - {}{}", marker, id_short.bright_yellow(), channel.name, status);
            }
        }
        println!();
        Ok(())
    }

    async fn cmd_channel(&mut self, args: &[&str]) -> Result<()> {
        let space_id = self.current_space.context("No space selected. Use: space <id>")?;

        if args.is_empty() {
            ui::print_error("Usage: channel create <name>  OR  channel <id>");
            return Ok(());
        }

        if args[0] == "create" {
            let name = args[1..].join(" ");
            if name.is_empty() {
                ui::print_error("Channel name cannot be empty");
                return Ok(());
            }

            let (channel, _op) = {
                let client = self.client.lock().await;
                client.create_channel(
                    space_id,
                    name.clone(),
                    Some(format!("Created by {}", self.username)),
                ).await?
            };

            self.current_channel = Some(channel.id);
            self.current_thread = None;

            ui::print_success(&format!("Created channel: {} ({})", name, hex::encode(&channel.id.0[..8])));
        } else {
            let prefix = args[0];
            let channels = {
                let client = self.client.lock().await;
                client.list_channels(&space_id).await
            };
            
            let matches: Vec<_> = channels.into_iter()
                .filter(|c| hex::encode(c.id.0).starts_with(prefix))
                .collect();

            match matches.len() {
                0 => ui::print_error(&format!("No channel found with ID prefix: {}", prefix)),
                1 => {
                    self.current_channel = Some(matches[0].id);
                    self.current_thread = None;
                    ui::print_success(&format!("Switched to channel: {}", matches[0].name));
                }
                _ => {
                    ui::print_error("Multiple channels match that prefix. Be more specific:");
                    for channel in matches {
                        println!("  {} - {}", hex::encode(&channel.id.0[..8]), channel.name);
                    }
                }
            }
        }

        Ok(())
    }

    async fn cmd_threads(&self) -> Result<()> {
        let channel_id = self.current_channel.context("No channel selected. Use: channel <id>")?;
        let threads = {
            let client = self.client.lock().await;
            client.list_threads(&channel_id).await
        };
        
        println!();
        if threads.is_empty() {
            ui::print_info("No threads yet. Create one with: thread create <title>");
        } else {
            println!("{} ({}):", "Threads".bright_cyan().bold(), threads.len());
            for thread in threads {
                let id_short = hex::encode(&thread.id.0[..8]);
                let marker = if Some(thread.id) == self.current_thread {
                    "→".bright_green()
                } else {
                    " ".normal()
                };
                let title = thread.title.as_deref().unwrap_or("Untitled");
                println!("  {} {} - {}", marker, id_short.bright_yellow(), title);
            }
        }
        println!();
        Ok(())
    }

    async fn cmd_thread(&mut self, args: &[&str]) -> Result<()> {
        let channel_id = self.current_channel.context("No channel selected. Use: channel <id>")?;

        if args.is_empty() {
            ui::print_error("Usage: thread create <title>  OR  thread <id>");
            return Ok(());
        }

        if args[0] == "create" {
            let title = args[1..].join(" ");
            if title.is_empty() {
                ui::print_error("Thread title cannot be empty");
                return Ok(());
            }

            let space_id = self.current_space.context("No space selected")?;
            let (thread, _op) = {
                let client = self.client.lock().await;
                client.create_thread(
                    space_id,
                    channel_id,
                    Some(title.clone()),
                    String::new(), // No initial content
                ).await?
            };

            self.current_thread = Some(thread.id);

            ui::print_success(&format!("Created thread: {} ({})", title, hex::encode(&thread.id.0[..8])));
        } else {
            let prefix = args[0];
            let threads = {
                let client = self.client.lock().await;
                client.list_threads(&channel_id).await
            };
            
            let matches: Vec<_> = threads.into_iter()
                .filter(|t| hex::encode(t.id.0).starts_with(prefix))
                .collect();

            match matches.len() {
                0 => ui::print_error(&format!("No thread found with ID prefix: {}", prefix)),
                1 => {
                    self.current_thread = Some(matches[0].id);
                    let title = matches[0].title.as_deref().unwrap_or("Untitled");
                    ui::print_success(&format!("Switched to thread: {}", title));
                }
                _ => {
                    ui::print_error("Multiple threads match that prefix. Be more specific:");
                    for thread in matches {
                        let title = thread.title.as_deref().unwrap_or("Untitled");
                        println!("  {} - {}", hex::encode(&thread.id.0[..8]), title);
                    }
                }
            }
        }

        Ok(())
    }

    async fn cmd_messages(&self) -> Result<()> {
        let thread_id = self.current_thread.context("No thread selected. Use: thread <id>")?;
        let messages = {
            let client = self.client.lock().await;
            client.list_messages(&thread_id).await
        };
        
        println!();
        if messages.is_empty() {
            ui::print_info("No messages yet. Send one with: send <text>");
        } else {
            println!("{} ({}):", "Messages".bright_cyan().bold(), messages.len());
            for msg in messages {
                let author_short = hex::encode(&msg.author.as_bytes()[..4]);
                let time_secs = msg.created_at / 1000;
                let deleted = if msg.deleted { " [deleted]".red() } else { "".normal() };
                println!();
                println!("  {} {} {} ({}){}",
                    "│".bright_black(),
                    author_short.bright_yellow(),
                    chrono::DateTime::from_timestamp(time_secs as i64, 0)
                        .map(|dt| dt.format("%H:%M:%S").to_string())
                        .unwrap_or_else(|| "?".to_string())
                        .bright_black(),
                    hex::encode(&msg.id.0[..4]).bright_black(),
                    deleted
                );
                println!("  {} {}", "│".bright_black(), msg.content);
            }
        }
        println!();
        Ok(())
    }

    async fn cmd_send(&mut self, text: &str) -> Result<()> {
        let space_id = self.current_space.context("No space selected")?;
        let thread_id = self.current_thread.context("No thread selected. Use: thread <id>")?;

        if text.is_empty() {
            ui::print_error("Message cannot be empty");
            return Ok(());
        }

        let (msg, _op) = {
            let client = self.client.lock().await;
            client.post_message(space_id, thread_id, text.to_string()).await?
        };
        
        ui::print_success(&format!("Message sent ({})", hex::encode(&msg.id.0[..4])));
        Ok(())
    }

    async fn cmd_invite(&self, args: &[&str]) -> Result<()> {
        let space_id = self.current_space.context("No space selected. Use: space <id>")?;

        if args.is_empty() {
            // List invites
            let invites = {
                let client = self.client.lock().await;
                client.list_invites(&space_id).await
            };
            
            println!();
            if invites.is_empty() {
                ui::print_info("No active invites. Create one with: invite create");
            } else {
                println!("{} ({}):", "Active Invites".bright_cyan().bold(), invites.len());
                for invite in invites {
                    let expires = if let Some(exp) = invite.expires_at {
                        let time_secs = exp / 1000;
                        chrono::DateTime::from_timestamp(time_secs as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "?".to_string())
                    } else {
                        "Never".to_string()
                    };
                    println!("  {} {} - Expires: {}", 
                        "Code:".bright_green(),
                        invite.code.bright_yellow(), 
                        expires
                    );
                }
            }
            println!();
        } else if args[0] == "create" {
            // Create invite
            let _op = {
                let client = self.client.lock().await;
                client.create_invite(space_id, None, None).await?
            };
            
            // Fetch the latest invites to get the code
            let invites = {
                let client = self.client.lock().await;
                client.list_invites(&space_id).await
            };
            
            if let Some(invite) = invites.last() {
                ui::print_success(&format!("Created invite code: {}", invite.code.bright_yellow()));
                println!();
                println!("  Share this code with others to invite them:");
                println!("  {} join {} {}", 
                    "$".bright_black(), 
                    hex::encode(&space_id.0[..8]).bright_yellow(),
                    invite.code.bright_yellow()
                );
                println!();
            } else {
                ui::print_success("Created invite");
            }
        } else {
            ui::print_error("Usage: invite  OR  invite create");
        }

        Ok(())
    }

    async fn cmd_join(&mut self, args: &[&str]) -> Result<()> {
        if args.len() < 2 {
            ui::print_error("Usage: join <space_id> <invite_code>  OR  join dht <space_id>");
            return Ok(());
        }

        if args[0] == "dht" {
            // Join from DHT
            let space_id_hex = args[1];
            let mut space_id_bytes = [0u8; 32];
            hex::decode_to_slice(space_id_hex, &mut space_id_bytes)
                .context("Invalid space ID hex")?;
            let space_id = SpaceId(space_id_bytes);

            ui::print_info(&format!("Joining Space from DHT: {}...", space_id_hex));
            
            let space = {
                let client = self.client.lock().await;
                client.join_space_from_dht(space_id).await?
            };

            self.current_space = Some(space.id);
            self.current_channel = None;
            self.current_thread = None;

            ui::print_success(&format!("Joined Space from DHT: {}", space.name));
            println!();
            println!("  Members: {}", space.members.len());
            println!("  Visibility: {:?}", space.visibility);
            println!();
        } else {
            // Join with invite code
            let space_id_hex = args[0];
            let invite_code = args[1];

            let mut space_id_bytes = [0u8; 32];
            hex::decode_to_slice(space_id_hex, &mut space_id_bytes)
                .context("Invalid space ID hex")?;
            let space_id = SpaceId(space_id_bytes);

            ui::print_info(&format!("Joining Space with invite code: {}...", invite_code));

            let _op = {
                let client = self.client.lock().await;
                client.join_with_invite(space_id, invite_code.to_string()).await?
            };

            self.current_space = Some(space_id);
            self.current_channel = None;
            self.current_thread = None;

            ui::print_success("Successfully joined Space!");
            println!();
            println!("  Note: You'll receive MLS Welcome message when an admin adds you");
            println!();
        }

        Ok(())
    }

    async fn cmd_upload(&self, args: &[&str]) -> Result<()> {
        let space_id = self.current_space.context("No space selected")?;

        if args.is_empty() {
            ui::print_error("Usage: upload <file_path>");
            return Ok(());
        }

        let file_path = args.join(" ");
        let data = std::fs::read(&file_path)
            .context("Failed to read file")?;

        let filename = std::path::Path::new(&file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        ui::print_info(&format!("Uploading {} ({} bytes)...", filename, data.len()));

        let metadata = {
            let client = self.client.lock().await;
            client.store_blob_for_space(
                &space_id,
                &data,
                None, // Auto-detect MIME type would go here
                Some(filename.clone()),
            ).await?
        };

        ui::print_success(&format!("Uploaded: {} (hash: {})", 
            filename,
            hex::encode(&metadata.hash.as_bytes()[..8])
        ));

        Ok(())
    }

    async fn cmd_refresh(&self) -> Result<()> {
        ui::print_info("Refreshing network state...");
        
        // Just a simple status check
        let peer_id = {
            let client = self.client.lock().await;
            client.network_peer_id().await
        };

        ui::print_success(&format!("Connected as peer: {}", &peer_id[..16]));
        Ok(())
    }
}

