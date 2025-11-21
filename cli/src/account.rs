//! Account management - loading/creating keypairs

use anyhow::{Context, Result};
use spaceway_core::crypto::Keypair;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct AccountFile {
    /// Username/nickname
    username: String,
    /// Ed25519 private key (32 bytes)
    #[serde(with = "serde_bytes")]
    private_key: Vec<u8>,
}

pub struct AccountManager {
    path: PathBuf,
    username: Option<String>,
}

impl AccountManager {
    pub fn new(path: PathBuf) -> Result<Self> {
        Ok(Self {
            path,
            username: None,
        })
    }

    pub fn username(&self) -> &str {
        self.username.as_deref().unwrap_or("unknown")
    }

    pub fn load_or_create(&mut self) -> Result<Keypair> {
        if self.path.exists() {
            self.load()
        } else {
            self.create()
        }
    }

    fn load(&mut self) -> Result<Keypair> {
        let data = fs::read(&self.path)
            .with_context(|| format!("Failed to read account file: {}", self.path.display()))?;

        let account: AccountFile = serde_json::from_slice(&data)
            .context("Failed to parse account file")?;

        self.username = Some(account.username);

        if account.private_key.len() != 32 {
            anyhow::bail!("Invalid private key length: expected 32 bytes, got {}", account.private_key.len());
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&account.private_key);

        let keypair = Keypair::from_bytes(&key_bytes)
            .context("Failed to create keypair from private key")?;

        println!("✓ Loaded account: {}", self.username());
        Ok(keypair)
    }

    fn create(&mut self) -> Result<Keypair> {
        println!("Creating new account...");

        // Get username from filename
        let filename = self.path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("user")
            .to_string();

        self.username = Some(filename.clone());

        // Generate new keypair
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let private_bytes = signing_key.to_bytes();

        let keypair = Keypair::from_bytes(&private_bytes)?;

        // Save to file
        let account = AccountFile {
            username: filename,
            private_key: private_bytes.to_vec(),
        };

        let json = serde_json::to_string_pretty(&account)?;

        // Create parent directory if needed
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.path, json)
            .with_context(|| format!("Failed to write account file: {}", self.path.display()))?;

        println!("✓ Created new account: {}", self.username());
        println!("✓ Saved to: {}", self.path.display());

        Ok(keypair)
    }
}
