# euro-focus

[![Crates.io](https://img.shields.io/crates/v/euro-focus.svg)](https://crates.io/crates/euro-focus)
[![Documentation](https://docs.rs/euro-focus/badge.svg)](https://docs.rs/euro-focus)
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
euro-focus = "0.6.0"
```

For async support:

```toml
[dependencies]
euro-focus = { version = "0.6.0", features = ["async"] }
tokio = { version = "1", features = ["full"] }
```

## Quick Start - Channel-Based

Subscribe to focus changes and receive them via a channel:

```rust
use euro_focus::subscribe_focus_changes;

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
use euro_focus::FocusTracker;
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

            // Perform async operations here
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
use euro_focus::{FocusTracker, FocusTrackerConfig, IconConfig};

let config = FocusTrackerConfig::new()
    .with_poll_interval_ms(50)           // Faster polling (default: 100ms)
    .with_icon_config(
        IconConfig::new().with_size(128) // Custom icon size
    );

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

For detailed API documentation, visit [docs.rs/euro-focus](https://docs.rs/euro-focus).

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
