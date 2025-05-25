# eur-activity Documentation

## Overview
The `eur-activity` crate provides timeline storage for capturing system state over time within the Eurora application. It implements a strategy pattern for handling different types of user activities, with specialized support for browser-based content tracking.

## Documentation Index

### 📋 [Critical Issues Analysis](./critical-issues-analysis.md)
Comprehensive analysis of critical issues that need immediate attention, including:
- Compilation failures and TODO implementations
- Error handling and stability issues
- Performance and memory concerns
- Security and configuration problems

**Key Findings:**
- Multiple `todo!()` implementations causing runtime panics
- Extensive use of `expect()` and `unwrap()` leading to crashes
- Missing timestamp implementations in snapshots
- Performance and memory concerns with image handling

### 🏗️ [Architecture Overview](./architecture-overview.md)
Detailed architectural documentation covering:
- Core components and design patterns
- Data flow and integration points
- Strategy pattern implementation
- Protocol buffer communication
- LLM and UI integration

**Key Components:**
- [`ActivityStrategy`](../src/lib.rs:171) trait for different activity types
- [`BrowserStrategy`](../src/browser_activity.rs:287) for browser content tracking
- [`ActivityAsset`](../src/lib.rs:43) and [`ActivitySnapshot`](../src/lib.rs:53) traits for data modeling
- gRPC communication with browser extensions

### 🛣️ [Implementation Roadmap](./implementation-roadmap.md)
Prioritized development plan organized in phases:

#### Phase 1: Critical Fixes (1-2 weeks)
- Implement all TODO methods
- Replace panic-prone error handling
- Add proper timestamp tracking

#### Phase 2: Core Functionality (2-3 weeks)
- Comprehensive error handling with timeouts
- Dynamic asset naming and metadata
- Asset processing pipeline documentation

#### Phase 3: Performance & Reliability (3-4 weeks)
- Memory management and image optimization
- Asset type registry and processing pipeline
- Concurrency improvements
- Monitoring and observability

#### Phase 4: Testing & Documentation (2-3 weeks)
- Comprehensive test suite (80% coverage target)
- Performance benchmarks
- Complete API documentation

### 🧪 [Testing Strategy](./testing-strategy.md)
Comprehensive testing approach including:
- Unit tests for all trait implementations
- Integration tests for gRPC communication
- Performance tests for memory and timing
- Concurrency and thread safety validation

**Quality Targets:**
- 80% minimum code coverage
- Sub-5-second asset collection
- Maximum 100MB memory per activity
- 99.9% reliability for normal operations

## Quick Start

### Current Status
⚠️ **WARNING:** The crate currently has critical issues preventing safe operation due to incomplete implementations. See [Critical Issues Analysis](./critical-issues-analysis.md) for details.

### Prerequisites
- Rust 2024 edition
- Protocol buffer compiler (protoc)
- Browser extension for content tracking

### Basic Usage (After Fixes)
```rust
use eur_activity::{select_strategy_for_process, Activity};

// Select appropriate strategy for a browser process
let strategy = select_strategy_for_process(
    "firefox",
    "Firefox Browser".to_string(),
    "base64_icon_data".to_string()
).await?;

// Create activity and collect assets
let mut activity = Activity::new(
    "Web Browsing".to_string(),
    "base64_icon".to_string(),
    "firefox".to_string(),
    vec![]
);

// Retrieve assets from the strategy
let assets = strategy.retrieve_assets().await?;
activity.assets = assets;
```

## Development Guidelines

### Code Quality Standards
- All new code must include unit tests
- Error handling must use `Result` types, not panics
- Public APIs must have comprehensive documentation
- All async operations must have timeout handling

### Performance Requirements
- Asset collection should complete within 5 seconds
- Memory usage should not exceed 100MB per activity
- Image processing should not block the main thread
- gRPC calls should have 10-second timeouts

### Testing Requirements
- Minimum 80% code coverage for new code
- All error paths must be tested
- Performance tests for memory and timing
- Integration tests for external dependencies

## Integration Points

### Browser Extensions
The crate communicates with browser extensions via gRPC to collect:
- YouTube video transcripts and frames (extension ID: `7c7b59bb-d44d-431a-9f4d-64240172e092`)
- Article content and metadata (extension ID: `None` for fallback processing)
- PDF annotations and highlights

Extension IDs are intentionally hardcoded as they identify specific asset processing pipelines within the application architecture.

### LLM Integration
Activities and assets are converted to LLM message format for AI processing:
- Text content for articles
- Image content with transcripts for videos
- Context chips for UI representation

### Database Integration
Future integration with `eur-personal-db` for:
- Activity persistence
- Timeline storage
- Search and filtering capabilities

## Contributing

### Before Contributing
1. Read the [Critical Issues Analysis](./critical-issues-analysis.md) to understand current problems
2. Review the [Implementation Roadmap](./implementation-roadmap.md) for prioritized tasks
3. Check the [Testing Strategy](./testing-strategy.md) for testing requirements

### Development Process
1. Implement incomplete TODO methods first
2. Add comprehensive error handling
3. Add unit tests for all new functionality
4. Update documentation for API changes
5. Run performance benchmarks for significant changes

### Pull Request Requirements
- All tests must pass
- Code coverage must not decrease
- Documentation must be updated
- Performance impact must be assessed

## Support and Resources

### Related Crates
- [`eur-proto`](../../proto/eur-proto/): Protocol buffer definitions
- [`eur-native-messaging`](../eur-native-messaging/): Browser extension communication
- [`eur-prompt-kit`](../../common/eur-prompt-kit/): LLM message construction
- [`eur-personal-db`](../eur-personal-db/): Data persistence

### External Dependencies
- **gRPC/Tonic:** For browser extension communication
- **Protocol Buffers:** For structured data exchange
- **Chrono:** For timestamp handling
- **Image:** For image processing and format conversion
- **TauRPC:** For frontend-backend communication

## License
This crate is part of the Eurora project. See the main project LICENSE file for details.

## Changelog
- **v0.1.0:** Initial implementation with browser activity tracking
- **Current:** Critical issues identified, requires fixes before production use

---

**Note:** This crate is currently in development and has critical issues that prevent safe operation. Please refer to the [Critical Issues Analysis](./critical-issues-analysis.md) and [Implementation Roadmap](./implementation-roadmap.md) before attempting to use or modify this code.