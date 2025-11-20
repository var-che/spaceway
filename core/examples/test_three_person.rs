//! Simple test program for 3-person Descord interaction
//!
//! Usage:
//!   Terminal 1: cargo run --bin descord-test -- --name alice --port 9001
//!   Terminal 2: cargo run --bin descord-test -- --name bob --port 9002 --connect /ip4/127.0.0.1/tcp/9001
//!   Terminal 3: cargo run --bin descord-test -- --name charlie --port 9003 --connect /ip4/127.0.0.1/tcp/9001

use anyhow::Result;
use clap::Parser;
use descord_core::{Client, ClientConfig, crypto::signing::Keypair};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Parser)]
struct Args {
    /// Your name
    #[arg(short, long)]
    name: String,
    
    /// Port to listen on
    #[arg(short, long, default_value = "9000")]
    port: u16,
    
    /// Peer address to connect to
    #[arg(short, long)]
    connect: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("\n╔══════════════════════════════════════╗");
    println!("║  Descord 3-Person Test               ║");
    println!("╚══════════════════════════════════════╝\n");
    
    // Create or load keypair
    let key_path = PathBuf::from(format!("{}.key", args.name));
    let keypair = if key_path.exists() {
        let bytes = std::fs::read(&key_path)?;
        let key_bytes: [u8; 32] = bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Invalid key file"))?;
        Keypair::from_bytes(&key_bytes)?
    } else {
        let keypair = Keypair::generate();
        std::fs::write(&key_path, keypair.to_bytes())?;
        keypair
    };
    
    let user_id = keypair.user_id();
    println!("👤 Name: {}", args.name);
    println!("🔑 User ID: {}", hex::encode(&user_id.as_bytes()[..8]));
    
    // Configure client
    let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", args.port);
    let data_dir = PathBuf::from(format!("{}-data", args.name));
    
    let config = ClientConfig {
        storage_path: data_dir,
        listen_addrs: vec![listen_addr.clone()],
        bootstrap_peers: if let Some(ref peer) = args.connect {
            vec![peer.clone()]
        } else {
            vec![]
        },
    };
    
    println!("🌐 Listening on: {}", listen_addr);
    if let Some(ref peer) = args.connect {
        println!("🔗 Connecting to: {}", peer);
    }
    
    // Start client
    println!("\n⏳ Starting client...");
    let client = Client::new(keypair, config)?;
    client.start().await?;
    
    println!("✅ Client started!");
    println!("\n📝 Commands:");
    println!("  create space <name>    - Create a new space");
    println!("  create channel <name>  - Create a channel in current space");
    println!("  create thread <title> <message> - Create a thread");
    println!("  send <message>         - Send a message to current thread");
    println!("  list spaces            - List all spaces");
    println!("  list channels          - List channels in current space");
    println!("  list threads           - List threads in current channel");
    println!("  list messages          - List messages in current thread");
    println!("  quit                   - Exit");
    println!();
    
    // Interactive loop
    let mut current_space = None;
    let mut current_channel = None;
    let mut current_thread = None;
    
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    
    loop {
        print!("{}> ", args.name);
        use std::io::Write;
        std::io::stdout().flush()?;
        
        line.clear();
        reader.read_line(&mut line).await?;
        let input = line.trim();
        
        if input.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = input.split_whitespace().collect();
        
        match parts.get(0).map(|s| *s) {
            Some("quit") | Some("exit") => {
                println!("👋 Goodbye!");
                break;
            }
            Some("create") => {
                match parts.get(1).map(|s| *s) {
                    Some("space") if parts.len() >= 3 => {
                        let name = parts[2..].join(" ");
                        match client.create_space(name.clone(), None).await {
                            Ok((space, _, _)) => {
                                current_space = Some(space.id);
                                println!("✅ Created space: {} (ID: {})", name, hex::encode(space.id.0.as_bytes()));
                            }
                            Err(e) => println!("❌ Error: {}", e),
                        }
                    }
                    Some("channel") if parts.len() >= 3 => {
                        if let Some(space_id) = current_space {
                            let name = parts[2..].join(" ");
                            match client.create_channel(space_id, name.clone(), None).await {
                                Ok((channel, _)) => {
                                    current_channel = Some(channel.id);
                                    println!("✅ Created channel: {}", name);
                                }
                                Err(e) => println!("❌ Error: {}", e),
                            }
                        } else {
                            println!("❌ No space selected. Create or select a space first.");
                        }
                    }
                    Some("thread") if parts.len() >= 4 => {
                        if let (Some(space_id), Some(channel_id)) = (current_space, current_channel) {
                            let title = parts[2].to_string();
                            let message = parts[3..].join(" ");
                            match client.create_thread(space_id, channel_id, Some(title.clone()), message).await {
                                Ok((thread, _)) => {
                                    current_thread = Some(thread.id);
                                    println!("✅ Created thread: {}", title);
                                }
                                Err(e) => println!("❌ Error: {}", e),
                            }
                        } else {
                            println!("❌ No channel selected. Create or select a channel first.");
                        }
                    }
                    _ => println!("❌ Usage: create <space|channel|thread> <name>"),
                }
            }
            Some("send") if parts.len() >= 2 => {
                if let (Some(space_id), Some(thread_id)) = (current_space, current_thread) {
                    let message = parts[1..].join(" ");
                    match client.post_message(space_id, thread_id, message.clone()).await {
                        Ok((msg, _)) => {
                            println!("✅ Sent: {}", message);
                        }
                        Err(e) => println!("❌ Error: {}", e),
                    }
                } else {
                    println!("❌ No thread selected. Create or select a thread first.");
                }
            }
            Some("list") => {
                match parts.get(1).map(|s| *s) {
                    Some("spaces") => {
                        let spaces = client.list_spaces().await;
                        println!("\n📚 Spaces ({}):", spaces.len());
                        for space in spaces {
                            let marker = if Some(space.id) == current_space { "→" } else { " " };
                            println!("  {} {} - {} members", marker, space.name, space.members.len());
                        }
                        println!();
                    }
                    Some("channels") => {
                        if let Some(space_id) = current_space {
                            let channels = client.list_channels(&space_id).await;
                            println!("\n📁 Channels ({}):", channels.len());
                            for channel in channels {
                                let marker = if Some(channel.id) == current_channel { "→" } else { " " };
                                println!("  {} {}", marker, channel.name);
                            }
                            println!();
                        } else {
                            println!("❌ No space selected");
                        }
                    }
                    Some("threads") => {
                        if let Some(channel_id) = current_channel {
                            let threads = client.list_threads(&channel_id).await;
                            println!("\n💬 Threads ({}):", threads.len());
                            for thread in threads {
                                let marker = if Some(thread.id) == current_thread { "→" } else { " " };
                                let title = thread.title.unwrap_or_else(|| "(no title)".to_string());
                                println!("  {} {} - {} messages", marker, title, thread.message_count);
                            }
                            println!();
                        } else {
                            println!("❌ No channel selected");
                        }
                    }
                    Some("messages") => {
                        if let Some(thread_id) = current_thread {
                            let messages = client.list_messages(&thread_id).await;
                            println!("\n💬 Messages ({}):", messages.len());
                            for msg in messages {
                                let author_short = hex::encode(&msg.author.as_bytes()[..4]);
                                println!("  [{}] {}", author_short, msg.content);
                            }
                            println!();
                        } else {
                            println!("❌ No thread selected");
                        }
                    }
                    _ => println!("❌ Usage: list <spaces|channels|threads|messages>"),
                }
            }
            Some(cmd) => {
                println!("❌ Unknown command: {}. Type 'quit' to exit or try 'create space MySpace'", cmd);
            }
            None => {}
        }
    }
    
    Ok(())
}
