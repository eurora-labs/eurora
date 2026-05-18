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
focus-tracker = "1.1.0"
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

## Ignoring focus events

Each platform exposes its own ignore-rule set. When a rule matches a focused
window, the tracker suppresses the event entirely: no `on_focus` callback,
no dedup-state update, and no icon work. Rule sets for the other platforms
are accepted by the builder (so cross-platform consumers don't need `cfg!`
scaffolding) and silently unused.

An `IgnoreRule` combines a process-name predicate **and** a window-title
predicate. A rule matches when **both** predicates match; the set matches
when **any** rule does.

```rust
use focus_tracker::{FocusTrackerConfig, IgnoreRule, WindowTitleMatch};

let config = FocusTrackerConfig::builder()
    .linux_ignore_rules([
        IgnoreRule::builder().process_name("firefox").build(),
        IgnoreRule::builder().process_name("chrome").build(),
        // Suppress "whatever" only when it has no title.
        IgnoreRule::builder()
            .process_name("whatever")
            .window_title(WindowTitleMatch::Missing)
            .build(),
    ])
    .macos_ignore_rules([
        IgnoreRule::builder().process_name("Firefox").build(),
        IgnoreRule::builder().process_name("Google Chrome").build(),
    ])
    .windows_ignore_rules([
        IgnoreRule::builder().process_name("firefox.exe").build(),
        IgnoreRule::builder().process_name("chrome.exe").build(),
    ])
    .build();
```

### Builder setters

`IgnoreRule::builder()` exposes two setters, both optional:

| Setter | Accepts | Default | Effect |
| ------ | ------- | ------- | ------ |
| `.process_name(s)` | any `Into<String>` | `ProcessNameMatch::Any` | matches the named process byte-exactly |
| `.window_title(m)` | a [`WindowTitleMatch`] | `WindowTitleMatch::Any` | restricts to titles matching `m` |

Pass `WindowTitleMatch::Missing` to match titleless windows (`None` **or**
`Some("")`), `Present` for any non-empty title, or `Exact(t)` for a
specific non-empty title. Omitting `.process_name(...)` matches every
process — useful for title-only rules like "ignore every titleless focus
event regardless of process":

```rust
use focus_tracker::{IgnoreRule, WindowTitleMatch};

let rule = IgnoreRule::builder()
    .window_title(WindowTitleMatch::Missing)
    .build();
```

### Matching is strict

Names and titles are matched **byte-exactly** against the values emitted by
the platform — no case folding, no `.exe` stripping, no basename
normalization. Provide every spelling you want to suppress.

| Platform | Source of `process_name` | Typical value |
| -------- | ------------------------ | ------------- |
| Linux    | `/proc/$pid/comm`, falling back to resolved `/proc/$pid/exe` | `firefox` (or a full path on fallback) |
| macOS    | `NSRunningApplication.localizedName()` | `Firefox` (localized, can differ by locale) |
| Windows  | `GetModuleBaseNameW` of the process's main module | `firefox.exe` |

A consequence: on Linux the comm name is capped at 15 bytes by the kernel,
and the exe-fallback path includes the full filesystem path. If you need
to ignore a process by both spellings, add a rule for each.

`with_title_missing` deliberately collapses `None` and `Some("")` into the
same "no title" category, because platforms disagree about which they emit
for a titleless window.

[`WindowTitleMatch`]: https://docs.rs/focus-tracker/latest/focus_tracker/enum.WindowTitleMatch.html

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
