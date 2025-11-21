///! Compression support for blob storage
///!
///! Phase 3 Feature: LZ4 compression before encryption to reduce storage size

use anyhow::{Context, Result};
use lz4::{EncoderBuilder, Decoder};
use std::io::{Read, Write};

/// Compress data using LZ4
///
/// LZ4 is chosen for its speed over gzip. We prioritize compression/decompression
/// speed over compression ratio since encryption adds constant overhead anyway.
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = EncoderBuilder::new()
        .level(4) // Fast compression (0-16, higher = more compression but slower)
        .build(Vec::new())
        .context("Failed to create LZ4 encoder")?;
    
    encoder.write_all(data)
        .context("Failed to write data to encoder")?;
    
    let (compressed, result) = encoder.finish();
    result.context("Failed to finish compression")?;
    
    Ok(compressed)
}

/// Decompress LZ4-compressed data
pub fn decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = Decoder::new(compressed)
        .context("Failed to create LZ4 decoder")?;
    
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)
        .context("Failed to decompress data")?;
    
    Ok(decompressed)
}

/// Calculate compression ratio (compressed size / original size)
///
/// Lower is better (e.g., 0.5 = 50% size reduction)
pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f32 {
    if original_size == 0 {
        return 1.0;
    }
    compressed_size as f32 / original_size as f32
}

/// Decide whether to compress based on data characteristics
///
/// Small data (<512 bytes) or already-compressed data (e.g., images) won't benefit
pub fn should_compress(data: &[u8], mime_type: Option<&str>) -> bool {
    // Don't compress tiny data (overhead not worth it)
    if data.len() < 512 {
        return false;
    }
    
    // Don't compress already-compressed formats
    if let Some(mime) = mime_type {
        let mime_lower = mime.to_lowercase();
        if mime_lower.contains("image/") 
            || mime_lower.contains("video/")
            || mime_lower.contains("audio/")
            || mime_lower.contains("zip")
            || mime_lower.contains("gzip")
            || mime_lower.contains("bzip2") {
            return false;
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compress_decompress_roundtrip() -> Result<()> {
        // Create data large enough to pass should_compress threshold (>512 bytes)
        let original_text = "Hello, world! This is a test message that should compress well because it has repeated patterns. ".repeat(10);
        let original = original_text.as_bytes();
        
        let compressed = compress(original)?;
        let decompressed = decompress(&compressed)?;
        
        assert_eq!(original, decompressed.as_slice());
        assert!(compressed.len() < original.len(), "Should compress repeated text");
        
        Ok(())
    }
    
    #[test]
    fn test_compression_ratio() {
        let ratio = compression_ratio(1000, 500);
        assert_eq!(ratio, 0.5);
        
        let ratio = compression_ratio(1000, 1000);
        assert_eq!(ratio, 1.0);
        
        let ratio = compression_ratio(0, 0);
        assert_eq!(ratio, 1.0);
    }
    
    #[test]
    fn test_should_compress_text() {
        // Create text large enough to pass threshold (>512 bytes)
        let text = "This is a long text message that should be compressed. ".repeat(15);
        assert!(should_compress(text.as_bytes(), Some("text/plain")));
        
        // Create JSON large enough (>512 bytes)
        let json_data: Vec<String> = (1..200).map(|i| format!(r#"{{"id": {}, "value": "data{}"}} "#, i, i)).collect();
        let json = format!("[{}]", json_data.join(", "));
        assert!(should_compress(json.as_bytes(), Some("application/json")));
    }
    
    #[test]
    fn test_should_not_compress_images() {
        let data = vec![0u8; 1024];
        assert!(!should_compress(&data, Some("image/png")));
        assert!(!should_compress(&data, Some("image/jpeg")));
        assert!(!should_compress(&data, Some("video/mp4")));
        assert!(!should_compress(&data, Some("audio/mp3")));
    }
    
    #[test]
    fn test_should_not_compress_small_data() {
        let small = vec![0u8; 100];
        assert!(!should_compress(&small, Some("text/plain")));
        
        let large = vec![0u8; 1000];
        assert!(should_compress(&large, Some("text/plain")));
    }
    
    #[test]
    fn test_compress_json() -> Result<()> {
        let json = r#"{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}, {"id": 3, "name": "Charlie"}], "timestamp": 1234567890}"#;
        
        let compressed = compress(json.as_bytes())?;
        let ratio = compression_ratio(json.len(), compressed.len());
        
        println!("Original: {} bytes", json.len());
        println!("Compressed: {} bytes", compressed.len());
        println!("Ratio: {:.2}", ratio);
        
        assert!(ratio < 1.0, "Should compress JSON");
        
        let decompressed = decompress(&compressed)?;
        assert_eq!(json.as_bytes(), decompressed.as_slice());
        
        Ok(())
    }
    
    #[test]
    fn test_compress_random_data() -> Result<()> {
        // Random data shouldn't compress well
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random: Vec<u8> = (0..1000).map(|_| rng.gen()).collect();
        
        let compressed = compress(&random)?;
        let ratio = compression_ratio(random.len(), compressed.len());
        
        // Random data might not compress, ratio could be >= 1.0
        println!("Random data compression ratio: {:.2}", ratio);
        
        let decompressed = decompress(&compressed)?;
        assert_eq!(random, decompressed);
        
        Ok(())
    }
}
