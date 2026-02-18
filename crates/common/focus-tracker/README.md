# focus-tracker

[![Crates.io](https://img.shields.io/crates/v/focus-tracker.svg)](https://crates.io/crates/focus-tracker)
[![Documentation](https://docs.rs/focus-tracker/badge.svg)](https://docs.rs/focus-tracker)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

A cross-platform focus tracker for Linux (X11), macOS, and Windows that monitors window focus changes and provides detailed information about the currently focused window.

## Features

-   Cross-platform support (Linux X11, macOS, Windows)
-   Real-time focus tracking
-   Window information (title, process name, PID)
-   Icon extraction with configurable sizes
-   Sync and async APIs
-   Configurable polling intervals
-   Graceful shutdown with stop signals

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
focus-tracker = "*"
```

For async support:

```toml
[dependencies]
focus-tracker = { version = "*", features = ["async"] }
tokio = { version = "1", features = ["full"] }
```

## Quick Start - Channel-Based

Subscribe to focus changes and receive them via a channel:

```rust
use focus_tracker::subscribe_focus_changes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let receiver = subscribe_focus_changes()?;

    while let Ok(window) = receiver.recv() {
        println!("Focused: {}",
            window.window_title.as_deref().unwrap_or("Unknown"));

        if let Some(process) = &window.process_name {
            println!("Process: {}", process);
        }
    }

    Ok(())
}
```

## Async Usage

For async/await workflows, use the async API with tokio:

```rust
use focus_tracker::FocusTracker;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = FocusTracker::new();
    let stop_signal = Arc::new(AtomicBool::new(false));

    tracker.track_focus_async_with_stop(
        |window| async move {
            println!("Focused: {}",
                window.window_title.as_deref().unwrap_or("Unknown"));

            some_async_function().await?;

            Ok(())
        },
        &stop_signal,
    ).await?;

    Ok(())
}
```

## Configuration

Customize behavior with `FocusTrackerConfig`:

```rust
use focus_tracker::{FocusTracker, FocusTrackerConfig, IconConfig};

let config = FocusTrackerConfig::builder()
    .poll_interval(std::time::Duration::from_millis(50))  // Faster polling (default: 100ms)
    .unwrap()
    .icon(IconConfig::builder().size(128).unwrap().build()) // Custom icon size
    .build();

let tracker = FocusTracker::with_config(config);
```

## Examples

Run the included examples:

```bash
# Channel-based focus tracking
cargo run --example basic

# Async focus tracking (requires async feature)
cargo run --example async --features async

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
