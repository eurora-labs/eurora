# EUR Activity - Extensible Activity Collection System

A flexible, extensible system for collecting activity data from various applications to reconstruct user workflows.

## Overview

The `eur-activity` crate provides a plugin-like architecture for collecting activity data from different applications. It uses a strategy pattern with dynamic registration to support various application types while maintaining extensibility for future additions.

## Architecture

### Core Components

```mermaid
graph TB
    A[StrategyRegistry] --> B[StrategyFactory]
    B --> C[ActivityStrategy]
    C --> D[ActivityAsset]
    C --> E[ActivitySnapshot]
    
    F[ProcessContext] --> A
    G[ActivityConfig] --> A
    
    H[BrowserStrategy] --> C
    I[DefaultStrategy] --> C
    J[FutureStrategy] --> C
```

### Key Traits

- **`ActivityStrategy`**: Core trait for collecting activity data from applications
- **`StrategyFactory`**: Factory pattern for creating strategy instances
- **`ActivityAsset`**: Represents contextual data (documents, videos, etc.)
- **`ActivitySnapshot`**: Represents temporal state captures

## Current Implementations

### Browser Strategy
Collects data from web browsers including:
- YouTube videos with transcripts
- Articles and web content
- Twitter/social media content
- PDF documents

### Default Strategy
Fallback strategy for unsupported applications that provides basic metadata collection.

## Asset Storage

The activity system now includes comprehensive asset storage functionality that allows you to save activity assets to disk for persistent storage and later reference in SQLite databases.

### Key Features

- **Type-safe asset saving**: Each asset type implements the `SaveableAsset` trait
- **Content deduplication**: Uses SHA-256 hashing to avoid storing duplicate content
- **Organized storage**: Assets can be organized by type in separate directories
- **Configurable storage**: Flexible configuration for storage location and behavior
- **Path generation**: Returns file paths suitable for SQLite storage
- **Async support**: All operations are async for better performance

### Storage Configuration

```rust
use eur_activity::{AssetStorage, StorageConfig};
use std::path::PathBuf;

// Create storage configuration
let config = StorageConfig {
    base_dir: PathBuf::from("./my_assets"),
    organize_by_type: true,        // Create youtube/, article/, etc. subdirectories
    use_content_hash: true,        // Enable content deduplication
    max_file_size: Some(50 * 1024 * 1024), // 50MB limit
};

let storage = AssetStorage::new(config);

// Or use the convenience constructor
let storage = AssetStorage::with_base_dir("./my_assets");
```

### Saving Assets

```rust
use eur_activity::{Activity, AssetStorage, YoutubeAsset, TranscriptLine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = AssetStorage::with_base_dir("./assets");
    
    // Create a YouTube asset
    let youtube_asset = YoutubeAsset::new(
        "yt-123".to_string(),
        "https://youtube.com/watch?v=example".to_string(),
        "Rust Tutorial".to_string(),
        vec![
            TranscriptLine {
                text: "Welcome to Rust programming".to_string(),
                start: 0.0,
                duration: 3.0,
            }
        ],
        120.5,
    );
    
    // Save individual asset
    let saved_info = youtube_asset.save_to_disk(&storage).await?;
    println!("Saved to: {}", saved_info.file_path.display());
    println!("Full path: {}", saved_info.absolute_path.display());
    
    // Save all assets in an activity
    let activity = Activity::new(
        "My Activity".to_string(),
        "icon".to_string(),
        "process".to_string(),
        vec![ActivityAsset::Youtube(youtube_asset)],
    );
    
    let saved_assets = activity.save_assets_to_disk(&storage).await?;
    for saved_asset in saved_assets {
        // Store the file_path in your SQLite database
        println!("Asset saved: {}", saved_asset.file_path.display());
    }
    
    Ok(())
}
```

### Storage Structure

With `organize_by_type: true`, assets are organized like this:

```
assets/
├── youtube/
│   ├── a1b2c3d4e5f6g7h8_Rust_Tutorial.json
│   └── f9e8d7c6b5a4g3h2_Advanced_Rust.json
├── article/
│   ├── 1a2b3c4d5e6f7g8h_Programming_Guide.json
│   └── 8h7g6f5e4d3c2b1a_Best_Practices.json
├── twitter/
│   └── 9i8h7g6f5e4d3c2b_Timeline_Capture.json
└── default/
    └── 2b3c4d5e6f7g8h9i_VS_Code_Session.json
```

### Content Deduplication

When `use_content_hash: true`, identical content is automatically deduplicated:

```rust
// These will result in the same file being used
let asset1 = YoutubeAsset::new(/* same content */);
let asset2 = YoutubeAsset::new(/* same content */);

let saved1 = asset1.save_to_disk(&storage).await?;
let saved2 = asset2.save_to_disk(&storage).await?;

// saved1.file_path == saved2.file_path (same file!)
assert_eq!(saved1.content_hash, saved2.content_hash);
```

### SQLite Integration

The returned file paths are perfect for storing in SQLite:

```sql
CREATE TABLE saved_assets (
    id INTEGER PRIMARY KEY,
    activity_id INTEGER,
    asset_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    content_hash TEXT,
    file_size INTEGER,
    saved_at DATETIME,
    FOREIGN KEY (activity_id) REFERENCES activities(id)
);
```

```rust
// Store in database
let saved_info = asset.save_to_disk(&storage).await?;
sqlx::query!(
    "INSERT INTO saved_assets (asset_type, file_path, content_hash, file_size, saved_at)
     VALUES (?, ?, ?, ?, ?)",
    saved_info.mime_type,
    saved_info.file_path.to_string_lossy(),
    saved_info.content_hash,
    saved_info.file_size as i64,
    saved_info.saved_at
).execute(&pool).await?;
```

## Usage

### Basic Usage

```rust
use eur_activity::{select_strategy_for_process, ProcessContext};
use ferrous_focus::IconData;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Select strategy for a process
    let strategy = select_strategy_for_process(
        "firefox",
        "Firefox Browser".to_string(),
        IconData::default(),
    ).await?;
    
    // Collect assets
    let mut strategy = strategy;
    let assets = strategy.retrieve_assets().await?;
    
    // Collect snapshots
    let snapshots = strategy.retrieve_snapshots().await?;
    
    Ok(())
}
```

### Advanced Configuration

```rust
use eur_activity::{
    ActivityConfigBuilder, StrategyConfig, SnapshotFrequency,
    get_registry, ProcessContext,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create custom configuration
    let config = ActivityConfigBuilder::new()
        .enable_collection(true)
        .default_collection_interval(Duration::from_secs(5))
        .max_assets_per_activity(15)
        .collect_content(true)
        .configure_strategy(
            "browser".to_string(),
            StrategyConfig {
                enabled: true,
                priority: 90,
                collection_interval: Duration::from_secs(3),
                asset_types: vec!["youtube".to_string(), "article".to_string()],
                snapshot_frequency: SnapshotFrequency::Interval(Duration::from_secs(10)),
                settings: std::collections::HashMap::new(),
            },
        )
        .build();
    
    // Use registry directly for more control
    let registry = get_registry();
    let mut registry_guard = registry.lock().await;
    
    let context = ProcessContext::new(
        "chrome".to_string(),
        "Google Chrome".to_string(),
        IconData::default(),
    );
    
    let strategy = registry_guard.select_strategy(&context).await?;
    
    Ok(())
}
```

## Extending the System

### Adding a New Strategy

1. **Implement the ActivityStrategy trait**:

```rust
use eur_activity::{ActivityStrategy, ActivityAsset, ActivitySnapshot};
use async_trait::async_trait;
use anyhow::Result;

pub struct VSCodeStrategy {
    name: String,
    process_name: String,
    // ... other fields
}

#[async_trait]
impl ActivityStrategy for VSCodeStrategy {
    async fn retrieve_assets(&mut self) -> Result<Vec<Box<dyn ActivityAsset>>> {
        // Collect open files, project context, etc.
        Ok(vec![])
    }
    
    async fn retrieve_snapshots(&mut self) -> Result<Vec<Box<dyn ActivitySnapshot>>> {
        // Collect cursor position, active file, git status, etc.
        Ok(vec![])
    }
    
    fn gather_state(&self) -> String {
        // Return current state as string
        String::new()
    }
    
    fn get_name(&self) -> &String { &self.name }
    fn get_icon(&self) -> &String { &self.name }
    fn get_process_name(&self) -> &String { &self.process_name }
}
```

2. **Create a StrategyFactory**:

```rust
use eur_activity::{
    StrategyFactory, ProcessContext, MatchScore, StrategyMetadata, StrategyCategory
};

pub struct VSCodeStrategyFactory;

#[async_trait]
impl StrategyFactory for VSCodeStrategyFactory {
    async fn create_strategy(&self, context: &ProcessContext) -> Result<Box<dyn ActivityStrategy>> {
        Ok(Box::new(VSCodeStrategy::new(context)?))
    }
    
    fn supports_process(&self, process_name: &str, _: Option<&str>) -> MatchScore {
        if process_name.contains("code") || process_name.contains("vscode") {
            MatchScore::HIGH
        } else {
            MatchScore::NO_MATCH
        }
    }
    
    fn get_metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            id: "vscode".to_string(),
            name: "VS Code Strategy".to_string(),
            version: "1.0.0".to_string(),
            description: "Collects activity from Visual Studio Code".to_string(),
            supported_processes: vec!["code".to_string(), "vscode".to_string()],
            category: StrategyCategory::Development,
        }
    }
}
```

3. **Register the factory**:

```rust
use eur_activity::get_registry;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = get_registry();
    let mut registry_guard = registry.lock().await;
    
    registry_guard.register_factory(Arc::new(VSCodeStrategyFactory::new()));
    
    Ok(())
}
```

### Custom Asset Types

```rust
use eur_activity::{ActivityAsset, ContextChip};
use ferrous_llm_core::Message;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct CodeFileAsset {
    pub file_path: String,
    pub language: String,
    pub content_preview: String,
    pub git_context: Option<String>,
}

impl ActivityAsset for CodeFileAsset {
    fn get_name(&self) -> &String {
        &self.file_path
    }
    
    fn get_icon(&self) -> Option<&String> {
        None
    }
    
    fn construct_message(&self) -> Message {
        // Create LLM message with code context
        Message::new(/* ... */)
    }
    
    fn get_context_chip(&self) -> Option<ContextChip> {
        Some(ContextChip {
            id: self.file_path.clone(),
            name: "code-file".to_string(),
            extension_id: "vscode-extension-id".to_string(),
            attrs: std::collections::HashMap::new(),
            icon: None,
            position: Some(0),
        })
    }
}
```

## Configuration System

### Global Configuration

```rust
use eur_activity::{ActivityConfig, GlobalConfig, PrivacyConfig};

let config = ActivityConfig {
    global: GlobalConfig {
        enabled: true,
        default_collection_interval: Duration::from_secs(3),
        max_assets_per_activity: 10,
        max_snapshots_per_activity: 100,
        privacy: PrivacyConfig {
            collect_content: true,
            anonymize_data: false,
            exclude_patterns: vec![
                r"password".to_string(),
                r"secret".to_string(),
            ],
            ignored_applications: vec!["sensitive-app".to_string()],
        },
    },
    strategies: HashMap::new(),
    applications: HashMap::new(),
};
```

### Strategy-Specific Configuration

```rust
use eur_activity::{StrategyConfig, SnapshotFrequency};

let browser_config = StrategyConfig {
    enabled: true,
    priority: 80,
    collection_interval: Duration::from_secs(3),
    asset_types: vec![
        "youtube".to_string(),
        "article".to_string(),
        "twitter".to_string(),
    ],
    snapshot_frequency: SnapshotFrequency::Interval(Duration::from_secs(5)),
    settings: HashMap::new(),
};
```

## Privacy and Security

The system includes comprehensive privacy controls:

- **Content Collection Control**: Choose between full content or metadata-only collection
- **Data Anonymization**: Automatic anonymization of sensitive data
- **Pattern Exclusion**: Regex patterns to exclude sensitive information
- **Application Ignoring**: Completely ignore specific applications
- **Per-Application Overrides**: Custom privacy settings per application

## Integration with Timeline

The activity system integrates seamlessly with `eur-timeline`:

```rust
use eur_timeline::TimelineManager;
use eur_activity::select_strategy_for_process;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut timeline = TimelineManager::new();
    timeline.start().await?;
    
    // The timeline automatically uses the activity system
    // to collect data from focused applications
    
    Ok(())
}
```

## Future Extensions

The architecture is designed to support:

### Planned Strategies
- **IDE Strategies**: VS Code, IntelliJ IDEA, Vim/Neovim
- **Communication Strategies**: Slack, Discord, Microsoft Teams
- **Productivity Strategies**: Notion, Obsidian, Roam Research
- **Design Strategies**: Figma, Adobe Creative Suite
- **Terminal Strategies**: Command line activity tracking
- **Media Strategies**: Spotify, VLC, other media players

### Advanced Features
- **Workflow Reconstruction**: Automatic detection of task boundaries and workflows
- **Context Linking**: Intelligent linking between related activities
- **Semantic Analysis**: Understanding of activity content and relationships
- **Plugin System**: Dynamic loading of strategy plugins
- **Cross-Platform Support**: Platform-specific optimizations

## Examples

### Asset Saving Example

Run the asset saving example to see the storage functionality in action:

```bash
cargo run --example asset_saving
```

This example demonstrates:
- Creating different types of assets (YouTube, Article, Twitter, Default)
- Configuring asset storage with different options
- Saving assets to disk with content deduplication
- Organizing assets by type
- Directory structure creation
- SQLite-ready file path generation

### Extensible Activity System Example

Run the comprehensive example:

```bash
cargo run --example extensible_activity_system
```

This example demonstrates:
- Registry initialization
- Configuration system usage
- Strategy selection for different processes
- Framework for future extensions

## Testing

Run the test suite:

```bash
cargo test
```

The tests cover:
- Strategy registry functionality
- Factory pattern implementation
- Configuration validation
- Browser strategy specifics
- Error handling

## Contributing

When adding new strategies:

1. Follow the established patterns
2. Add comprehensive tests
3. Update documentation
4. Consider privacy implications
5. Ensure cross-platform compatibility

## License

This crate is part of the Eurora project and follows the same licensing terms.