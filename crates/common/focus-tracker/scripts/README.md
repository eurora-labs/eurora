# Headless Display Scripts

This directory contains scripts for setting up headless display environments for testing focus-tracker on Linux systems.

## X11 Headless Setup

Use `start_x11_headless.sh` to start a headless X11 display with Xvfb and a lightweight window manager.

### Usage

```bash
# Start headless X11 on display :99 (default)
./scripts/start_x11_headless.sh

# Start on a specific display number
./scripts/start_x11_headless.sh 100

# Run in daemon mode (background)
./scripts/start_x11_headless.sh 99 --daemon
```

### Requirements

-   `xvfb` - Virtual framebuffer X server
-   `openbox` - Lightweight window manager

Install on Ubuntu/Debian:

```bash
sudo apt-get install xvfb openbox
```

### Environment

After starting, export the display:

```bash
export DISPLAY=:99
```

## Wayland Headless Setup

Use `start_wayland_headless.sh` to start a headless Wayland compositor using Sway.

### Usage

```bash
# Start headless Wayland
./scripts/start_wayland_headless.sh

# Run in daemon mode (background)
./scripts/start_wayland_headless.sh --daemon
```

### Requirements

-   `sway` - Wayland compositor
-   `swayidle` - Idle management daemon

Install on Ubuntu/Debian:

```bash
sudo apt-get install sway swayidle
```

### Environment

After starting, export the Wayland display:

```bash
export WAYLAND_DISPLAY=wayland-test
```

## Testing Integration

These scripts are designed to work with the focus-tracker integration tests:

```bash
# Test with X11
export INTEGRATION_TEST=1
export X11=1
./scripts/start_x11_headless.sh 99 --daemon &
export DISPLAY=:99
cargo test --test integration_basic

# Test with Wayland
export INTEGRATION_TEST=1
export WAYLAND=1
./scripts/start_wayland_headless.sh --daemon &
export WAYLAND_DISPLAY=wayland-test
cargo test --test integration_basic
```

## CI/CD Usage

For continuous integration, you can use these scripts to set up the display environment before running tests:

```bash
# In your CI script
./scripts/start_x11_headless.sh 99 --daemon
export DISPLAY=:99
export INTEGRATION_TEST=1
export X11=1
cargo test --test integration_basic
```

## Troubleshooting

### X11 Issues

-   **"Xvfb: command not found"**: Install the `xvfb` package
-   **"openbox: command not found"**: Install the `openbox` package
-   **Display already in use**: Try a different display number
-   **Permission denied**: Make sure the script is executable (`chmod +x`)

### Wayland Issues

-   **"sway: command not found"**: Install the `sway` package
-   **Sway fails to start**: Check that you're not already running a Wayland session
-   **No input devices**: This is expected in headless mode (WLR_LIBINPUT_NO_DEVICES=1)

### General

-   **Tests skip**: Make sure `INTEGRATION_TEST=1` is set
-   **No focus events**: Verify the display environment is properly set up
-   **Permission errors**: Run with appropriate permissions for display access
