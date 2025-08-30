//! Example demonstrating asset saving functionality
//!
//! This example shows how to:
//! - Create different types of assets
//! - Configure asset storage
//! - Save assets to disk
//! - Retrieve saved asset information

use eur_activity::{
    Activity, ActivityAsset, ActivityStorage, ActivityStorageConfig, ArticleAsset,
    AssetFunctionality, DefaultAsset, TranscriptLine, TwitterAsset, TwitterContextType,
    TwitterTweet, YoutubeAsset, types::SaveFunctionality,
};
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("🚀 Asset Saving Example");
    println!("========================");

    // Create a temporary directory for this example
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path().to_path_buf();

    println!("📁 Using temporary directory: {}", base_path.display());

    // Configure asset storage
    let storage_config = ActivityStorageConfig {
        base_dir: base_path.clone(),
        organize_by_type: true,
        use_content_hash: true,
        max_file_size: Some(10 * 1024 * 1024), // 10MB limit
    };

    let storage = ActivityStorage::new(storage_config);

    println!("⚙️  Storage configured with content hashing and type organization");

    // Create sample assets
    let assets = create_sample_assets();

    println!("📦 Created {} sample assets", assets.len());

    // Create an activity with these assets
    let activity = Activity::new(
        "Example Activity".to_string(),
        "example-icon".to_string(),
        "example-process".to_string(),
        assets,
    );

    // Save all assets to disk
    println!("\n💾 Saving assets to disk...");
    let saved_assets = activity.save_assets_to_disk(&storage).await?;

    println!("✅ Successfully saved {} assets:", saved_assets.len());

    for (i, saved_asset) in saved_assets.iter().enumerate() {
        println!(
            "  {}. {} -> {}",
            i + 1,
            saved_asset.file_path.display(),
            saved_asset.absolute_path.display()
        );
        println!(
            "     Size: {} bytes, Hash: {}",
            saved_asset.file_size,
            saved_asset.content_hash.as_deref().unwrap_or("none")
        );
    }

    // Demonstrate individual asset saving
    println!("\n🔍 Saving individual asset by index...");
    if let Some(saved_info) = activity.save_asset_by_index(0, &storage).await? {
        println!("✅ Saved asset: {}", saved_info.file_path.display());
        println!("   This should be the same file due to content deduplication!");
    }

    // Show directory structure
    println!("\n📂 Directory structure:");
    show_directory_structure(&base_path, 0).await?;

    // Demonstrate storage configuration options
    println!("\n🔧 Testing different storage configurations...");

    // Configuration without content hashing
    let no_hash_config = ActivityStorageConfig {
        base_dir: base_path.join("no_hash"),
        organize_by_type: false,
        use_content_hash: false,
        max_file_size: None,
    };

    let no_hash_storage = ActivityStorage::new(no_hash_config);

    // Save the same YouTube asset with different config
    if let Some(youtube_asset) = activity.assets.first() {
        let saved_info = youtube_asset.save_to_disk(&no_hash_storage).await?;
        println!(
            "📁 Saved without hashing: {}",
            saved_info.file_path.display()
        );
    }

    println!("\n🎉 Example completed successfully!");
    println!("💡 Check the temporary directory to see the saved files:");
    println!("   {}", base_path.display());

    // Keep the temp directory around for inspection
    std::mem::forget(temp_dir);

    Ok(())
}

fn create_sample_assets() -> Vec<ActivityAsset> {
    vec![
        // YouTube asset
        ActivityAsset::YoutubeAsset(YoutubeAsset::new(
            "yt-123".to_string(),
            "https://youtube.com/watch?v=example".to_string(),
            "How to Build Amazing Rust Applications".to_string(),
            vec![
                TranscriptLine {
                    text: "Welcome to this tutorial on Rust programming".to_string(),
                    start: 0.0,
                    duration: 3.5,
                },
                TranscriptLine {
                    text: "Today we'll learn about building applications".to_string(),
                    start: 3.5,
                    duration: 4.0,
                },
                TranscriptLine {
                    text: "Rust is a systems programming language".to_string(),
                    start: 7.5,
                    duration: 3.0,
                },
            ],
            45.2,
        )),

        // Article asset
        ActivityAsset::ArticleAsset(ArticleAsset::new(
            "article-456".to_string(),
            "https://example.com/rust-guide".to_string(),
            "The Complete Guide to Rust Programming".to_string(),
            "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety. It accomplishes these goals by being memory safe without using garbage collection.".to_string(),
            Some("Jane Developer".to_string()),
            Some("2024-01-15".to_string()),
        )),

        // Twitter asset
        ActivityAsset::TwitterAsset(TwitterAsset::new(
            "twitter-789".to_string(),
            "https://twitter.com/rustlang".to_string(),
            "Rust Language Updates".to_string(),
            vec![
                TwitterTweet::new(
                    "🦀 Rust 1.75 is now available! Check out the new features and improvements.".to_string(),
                    Some("rustlang".to_string()),
                    Some("2024-01-15T10:00:00Z".to_string()),
                ),
                TwitterTweet::new(
                    "The #RustLang community continues to grow! 💪 #programming #rust".to_string(),
                    Some("rustdev".to_string()),
                    Some("2024-01-15T11:30:00Z".to_string()),
                ),
            ],
            TwitterContextType::Timeline,
        )),

        // Default asset
        ActivityAsset::DefaultAsset(
            DefaultAsset::new(
                "default-101".to_string(),
                "VS Code - Rust Project".to_string(),
                Some("vscode-icon".to_string()),
                Some("Working on a Rust project in Visual Studio Code".to_string()),
            )
            .with_metadata("project_name".to_string(), "my-rust-app".to_string())
            .with_metadata("files_open".to_string(), "3".to_string())
            .with_metadata("git_branch".to_string(), "feature/asset-saving".to_string())
        ),
    ]
}

async fn show_directory_structure(
    path: &PathBuf,
    depth: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let indent = "  ".repeat(depth);

    if path.is_dir() {
        println!(
            "{}📁 {}",
            indent,
            path.file_name().unwrap_or_default().to_string_lossy()
        );

        let mut entries = tokio::fs::read_dir(path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                Box::pin(show_directory_structure(&entry_path, depth + 1)).await?;
            } else {
                let metadata = tokio::fs::metadata(&entry_path).await?;
                println!(
                    "{}📄 {} ({} bytes)",
                    "  ".repeat(depth + 1),
                    entry_path.file_name().unwrap_or_default().to_string_lossy(),
                    metadata.len()
                );
            }
        }
    }

    Ok(())
}
