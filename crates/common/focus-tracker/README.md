# focus-tracker

[![Crates.io](https://img.shields.io/crates/v/focus-tracker.svg)](https://crates.io/crates/focus-tracker)
[![Documentation](https://docs.rs/focus-tracker/badge.svg)](https://docs.rs/focus-tracker)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

A cross-platform focus tracker for Linux (X11), macOS, and Windows that monitors window focus changes and provides detailed information about the currently focused window.

## Features

-   Cross-platform support (Linux X11, macOS, Windows)
-   Real-time focus tracking with automatic deduplication
-   Window information (title, process name, PID)
-   Icon extraction with configurable sizes and bounded cache
-   Async API with tokio
-   Configurable polling intervals
-   Graceful shutdown with stop signals

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
focus-tracker = "1.0.0"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

Track focus changes using the async API with tokio:

```rust
use focus_tracker::FocusTracker;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = FocusTracker::builder().build();
    let stop_signal = Arc::new(AtomicBool::new(false));

    tracker
        .track_focus()
        .on_focus(|window| async move {
            println!("Focused: {}",
                window.window_title.as_deref().unwrap_or("Unknown"));
            Ok(())
        })
        .stop_signal(&stop_signal)
        .call()
        .await?;

    Ok(())
}
```

## Configuration

Customize behavior with `FocusTrackerConfig`:

```rust
use focus_tracker::{FocusTracker, FocusTrackerConfig, IconConfig};

let config = FocusTrackerConfig::builder()
    .poll_interval(std::time::Duration::from_millis(50)).unwrap()  // Faster polling (default: 100ms)
    .icon(IconConfig::builder().size(128).unwrap().build())        // Custom icon size (default: 128)
    .icon_cache_capacity(32).unwrap()                              // Bounded icon cache (default: 64)
    .build();

let tracker = FocusTracker::builder().config(config).build();
```

## Examples

Run the included examples:

```bash
# Basic focus tracking
cargo run --example basic

# Advanced example with icon saving and statistics
cargo run --example advanced
```

## Platform Support

| Platform | Window System | Status           |
| -------- | ------------- | ---------------- |
| Linux    | X11           | ✅ Full support  |
| Linux    | Wayland       | ❌ Not supported |
| macOS    | Cocoa         | ✅ Full support  |
| Windows  | Win32 API     | ✅ Full support  |

### Platform Notes

-   **Linux X11**: Full support
-   **Linux Wayland**: Not supported (technical limitations)
-   **macOS**: Requires accessibility permissions
-   **Windows**: Full support on Windows 7+

## System Requirements

### macOS

Accessibility permissions required. Grant in: System Preferences > Security & Privacy > Accessibility

### Linux

X11 development libraries required (pre-installed on most distributions)

### Windows

No additional requirements

## API Documentation

For detailed API documentation, visit [docs.rs/focus-tracker](https://docs.rs/focus-tracker).

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
