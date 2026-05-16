use bon::bon;

use crate::{FocusTrackerError, FocusTrackerResult, IgnoredProcesses};
use std::time::Duration;

fn validate_icon_size(size: u32) -> FocusTrackerResult<u32> {
    if size == 0 {
        return Err(FocusTrackerError::InvalidConfig {
            reason: "icon size cannot be zero".into(),
        });
    }
    if size > 512 {
        return Err(FocusTrackerError::InvalidConfig {
            reason: "icon size cannot be greater than 512 pixels".into(),
        });
    }
    Ok(size)
}

fn validate_poll_interval(interval: Duration) -> FocusTrackerResult<Duration> {
    if interval.is_zero() {
        return Err(FocusTrackerError::InvalidConfig {
            reason: "poll interval cannot be zero".into(),
        });
    }
    if interval > Duration::from_secs(10) {
        return Err(FocusTrackerError::InvalidConfig {
            reason: "poll interval cannot be greater than 10 seconds".into(),
        });
    }
    Ok(interval)
}

fn validate_icon_cache_capacity(capacity: usize) -> FocusTrackerResult<usize> {
    if capacity == 0 {
        return Err(FocusTrackerError::InvalidConfig {
            reason: "icon cache capacity cannot be zero".into(),
        });
    }
    Ok(capacity)
}

#[derive(Debug, Clone)]
pub struct IconConfig {
    pub size: Option<u32>,
    pub filter_type: image::imageops::FilterType,
}

impl Default for IconConfig {
    fn default() -> Self {
        Self {
            size: None,
            filter_type: image::imageops::FilterType::Lanczos3,
        }
    }
}

#[bon]
impl IconConfig {
    /// Creates a new icon configuration using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use focus_tracker_core::IconConfig;
    ///
    /// // Default config (no custom size, Lanczos3 filter)
    /// let config = IconConfig::builder().build();
    ///
    /// // Custom 64×64 icon size
    /// let config = IconConfig::builder()
    ///     .size(64)
    ///     .unwrap()
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        #[builder(with = |size: u32| -> Result<_, FocusTrackerError> {
            validate_icon_size(size)
        })]
        size: Option<u32>,

        #[builder(default = image::imageops::FilterType::Lanczos3)]
        filter_type: image::imageops::FilterType,
    ) -> Self {
        Self { size, filter_type }
    }
}

impl IconConfig {
    #[must_use]
    pub fn get_size_or_default(&self) -> u32 {
        self.size.unwrap_or(128)
    }
}

#[derive(Debug, Clone)]
pub struct FocusTrackerConfig {
    pub poll_interval: Duration,
    pub icon: IconConfig,
    pub icon_cache_capacity: usize,
    /// Process names to ignore when running on Linux.
    ///
    /// Consulted only by the Linux tracker; ignored on other platforms.
    /// Matched byte-exactly against [`FocusedWindow::process_name`].
    ///
    /// [`FocusedWindow::process_name`]: crate::FocusedWindow::process_name
    pub linux_ignored_processes: IgnoredProcesses,
    /// Process names to ignore when running on macOS.
    ///
    /// Consulted only by the macOS tracker; ignored on other platforms.
    /// Matched byte-exactly against [`FocusedWindow::process_name`].
    ///
    /// [`FocusedWindow::process_name`]: crate::FocusedWindow::process_name
    pub macos_ignored_processes: IgnoredProcesses,
    /// Process names to ignore when running on Windows.
    ///
    /// Consulted only by the Windows tracker; ignored on other platforms.
    /// Matched byte-exactly against [`FocusedWindow::process_name`].
    ///
    /// [`FocusedWindow::process_name`]: crate::FocusedWindow::process_name
    pub windows_ignored_processes: IgnoredProcesses,
}

impl Default for FocusTrackerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            icon: IconConfig::default(),
            icon_cache_capacity: 64,
            linux_ignored_processes: IgnoredProcesses::default(),
            macos_ignored_processes: IgnoredProcesses::default(),
            windows_ignored_processes: IgnoredProcesses::default(),
        }
    }
}

impl FocusTrackerConfig {
    /// Returns the ignore list that applies to the current build target.
    #[must_use]
    pub fn ignored_processes_for_current_platform(&self) -> &IgnoredProcesses {
        #[cfg(target_os = "linux")]
        {
            &self.linux_ignored_processes
        }
        #[cfg(target_os = "macos")]
        {
            &self.macos_ignored_processes
        }
        #[cfg(target_os = "windows")]
        {
            &self.windows_ignored_processes
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Fall back to a stable reference so other platforms compile.
            static EMPTY: std::sync::OnceLock<IgnoredProcesses> = std::sync::OnceLock::new();
            EMPTY.get_or_init(IgnoredProcesses::default)
        }
    }
}

#[bon]
impl FocusTrackerConfig {
    /// Creates a new focus tracker configuration using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use focus_tracker_core::{FocusTrackerConfig, IconConfig};
    /// use std::time::Duration;
    ///
    /// let config = FocusTrackerConfig::builder()
    ///     .poll_interval(Duration::from_millis(50))
    ///     .unwrap()
    ///     .icon(IconConfig::builder().size(64).unwrap().build())
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        #[builder(
            default = Duration::from_millis(100),
            with = |interval: Duration| -> Result<_, FocusTrackerError> {
                validate_poll_interval(interval)
            },
        )]
        poll_interval: Duration,
        #[builder(default)] icon: IconConfig,
        #[builder(
            default = 64,
            with = |capacity: usize| -> Result<_, FocusTrackerError> {
                validate_icon_cache_capacity(capacity)
            },
        )]
        icon_cache_capacity: usize,
        #[builder(
            default,
            with = |names: impl IntoIterator<Item: Into<String>>| IgnoredProcesses::new(names),
        )]
        linux_ignored_processes: IgnoredProcesses,
        #[builder(
            default,
            with = |names: impl IntoIterator<Item: Into<String>>| IgnoredProcesses::new(names),
        )]
        macos_ignored_processes: IgnoredProcesses,
        #[builder(
            default,
            with = |names: impl IntoIterator<Item: Into<String>>| IgnoredProcesses::new(names),
        )]
        windows_ignored_processes: IgnoredProcesses,
    ) -> Self {
        Self {
            poll_interval,
            icon,
            icon_cache_capacity,
            linux_ignored_processes,
            macos_ignored_processes,
            windows_ignored_processes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_icon_config() {
        let config = IconConfig::default();
        assert_eq!(config.size, None);
        assert_eq!(config.get_size_or_default(), 128);
    }

    #[test]
    fn icon_builder_defaults() {
        let config = IconConfig::builder().build();
        assert_eq!(config.size, None);
        assert_eq!(config.get_size_or_default(), 128);
    }

    #[test]
    fn icon_builder_with_size() {
        let config = IconConfig::builder().size(256).unwrap().build();
        assert_eq!(config.size, Some(256));
        assert_eq!(config.get_size_or_default(), 256);
    }

    #[test]
    fn icon_builder_max_size() {
        let config = IconConfig::builder().size(512).unwrap().build();
        assert_eq!(config.size, Some(512));
    }

    #[test]
    fn icon_builder_min_size() {
        let config = IconConfig::builder().size(1).unwrap().build();
        assert_eq!(config.size, Some(1));
    }

    #[test]
    fn icon_builder_zero_size_errors() {
        assert!(IconConfig::builder().size(0).is_err());
    }

    #[test]
    fn icon_builder_oversized_errors() {
        assert!(IconConfig::builder().size(513).is_err());
        assert!(IconConfig::builder().size(1024).is_err());
    }

    #[test]
    fn icon_builder_custom_filter() {
        let config = IconConfig::builder()
            .filter_type(image::imageops::FilterType::Nearest)
            .build();
        assert!(matches!(
            config.filter_type,
            image::imageops::FilterType::Nearest
        ));
    }

    #[test]
    fn default_config() {
        let config = FocusTrackerConfig::default();
        assert_eq!(config.poll_interval, Duration::from_millis(100));
        assert_eq!(config.icon.size, None);
        assert_eq!(config.icon_cache_capacity, 64);
    }

    #[test]
    fn config_builder_defaults() {
        let config = FocusTrackerConfig::builder().build();
        assert_eq!(config.poll_interval, Duration::from_millis(100));
        assert_eq!(config.icon.size, None);
        assert_eq!(config.icon_cache_capacity, 64);
    }

    #[test]
    fn config_builder_icon_cache_capacity() {
        let config = FocusTrackerConfig::builder()
            .icon_cache_capacity(128)
            .unwrap()
            .build();
        assert_eq!(config.icon_cache_capacity, 128);
    }

    #[test]
    fn config_builder_zero_cache_capacity_errors() {
        assert!(
            FocusTrackerConfig::builder()
                .icon_cache_capacity(0)
                .is_err()
        );
    }

    #[test]
    fn config_builder_poll_interval() {
        let config = FocusTrackerConfig::builder()
            .poll_interval(Duration::from_millis(250))
            .unwrap()
            .build();
        assert_eq!(config.poll_interval, Duration::from_millis(250));
    }

    #[test]
    fn config_builder_max_interval() {
        let config = FocusTrackerConfig::builder()
            .poll_interval(Duration::from_secs(10))
            .unwrap()
            .build();
        assert_eq!(config.poll_interval, Duration::from_secs(10));
    }

    #[test]
    fn config_builder_zero_interval_errors() {
        assert!(
            FocusTrackerConfig::builder()
                .poll_interval(Duration::ZERO)
                .is_err()
        );
    }

    #[test]
    fn config_builder_large_interval_errors() {
        assert!(
            FocusTrackerConfig::builder()
                .poll_interval(Duration::from_secs(11))
                .is_err()
        );
    }

    #[test]
    fn config_builder_with_icon() {
        let icon = IconConfig::builder().size(64).unwrap().build();
        let config = FocusTrackerConfig::builder().icon(icon).build();
        assert_eq!(config.icon.size, Some(64));
    }

    #[test]
    fn config_builder_full() {
        let config = FocusTrackerConfig::builder()
            .poll_interval(Duration::from_millis(50))
            .unwrap()
            .icon(IconConfig::builder().size(64).unwrap().build())
            .build();

        assert_eq!(config.poll_interval, Duration::from_millis(50));
        assert_eq!(config.icon.size, Some(64));
        assert_eq!(config.icon.get_size_or_default(), 64);
    }

    #[test]
    fn config_default_ignore_lists_are_empty() {
        let config = FocusTrackerConfig::default();
        assert!(config.linux_ignored_processes.is_empty());
        assert!(config.macos_ignored_processes.is_empty());
        assert!(config.windows_ignored_processes.is_empty());
    }

    #[test]
    fn config_builder_per_platform_ignore_lists() {
        let config = FocusTrackerConfig::builder()
            .linux_ignored_processes(["firefox"])
            .macos_ignored_processes(["Firefox"])
            .windows_ignored_processes(["firefox.exe", "chrome.exe"])
            .build();

        assert!(config.linux_ignored_processes.contains("firefox"));
        assert!(!config.linux_ignored_processes.contains("firefox.exe"));

        assert!(config.macos_ignored_processes.contains("Firefox"));
        assert!(!config.macos_ignored_processes.contains("firefox"));

        assert_eq!(config.windows_ignored_processes.len(), 2);
        assert!(config.windows_ignored_processes.contains("firefox.exe"));
        assert!(config.windows_ignored_processes.contains("chrome.exe"));
    }

    #[test]
    fn config_builder_accepts_string_and_str() {
        let config = FocusTrackerConfig::builder()
            .windows_ignored_processes([String::from("a"), String::from("b")])
            .build();
        assert_eq!(config.windows_ignored_processes.len(), 2);

        let config = FocusTrackerConfig::builder()
            .windows_ignored_processes(["a", "b"])
            .build();
        assert_eq!(config.windows_ignored_processes.len(), 2);
    }

    #[test]
    fn config_current_platform_selector_matches_target() {
        let config = FocusTrackerConfig::builder()
            .linux_ignored_processes(["lin"])
            .macos_ignored_processes(["mac"])
            .windows_ignored_processes(["win"])
            .build();

        let current = config.ignored_processes_for_current_platform();
        #[cfg(target_os = "linux")]
        assert!(current.contains("lin"));
        #[cfg(target_os = "macos")]
        assert!(current.contains("mac"));
        #[cfg(target_os = "windows")]
        assert!(current.contains("win"));
    }
}
