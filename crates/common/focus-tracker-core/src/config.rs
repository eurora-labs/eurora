use bon::bon;

use crate::{FocusTrackerError, FocusTrackerResult, IgnoreRule, IgnoreRules};
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
    /// Ignore rules applied when running on Linux.
    ///
    /// Consulted only by the Linux tracker; ignored on other platforms.
    /// See [`IgnoreRule`] for the matcher semantics.
    pub linux_ignore_rules: IgnoreRules,
    /// Ignore rules applied when running on macOS.
    ///
    /// Consulted only by the macOS tracker; ignored on other platforms.
    /// See [`IgnoreRule`] for the matcher semantics.
    pub macos_ignore_rules: IgnoreRules,
    /// Ignore rules applied when running on Windows.
    ///
    /// Consulted only by the Windows tracker; ignored on other platforms.
    /// See [`IgnoreRule`] for the matcher semantics.
    pub windows_ignore_rules: IgnoreRules,
}

impl Default for FocusTrackerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            icon: IconConfig::default(),
            icon_cache_capacity: 64,
            linux_ignore_rules: IgnoreRules::default(),
            macos_ignore_rules: IgnoreRules::default(),
            windows_ignore_rules: IgnoreRules::default(),
        }
    }
}

impl FocusTrackerConfig {
    /// Returns the ignore rules that apply to the current build target.
    #[must_use]
    pub fn ignore_rules_for_current_platform(&self) -> &IgnoreRules {
        #[cfg(target_os = "linux")]
        {
            &self.linux_ignore_rules
        }
        #[cfg(target_os = "macos")]
        {
            &self.macos_ignore_rules
        }
        #[cfg(target_os = "windows")]
        {
            &self.windows_ignore_rules
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Fall back to a stable reference so other platforms compile.
            static EMPTY: std::sync::OnceLock<IgnoreRules> = std::sync::OnceLock::new();
            EMPTY.get_or_init(IgnoreRules::default)
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
    /// use focus_tracker_core::{FocusTrackerConfig, IconConfig, IgnoreRule, WindowTitleMatch};
    /// use std::time::Duration;
    ///
    /// let config = FocusTrackerConfig::builder()
    ///     .poll_interval(Duration::from_millis(50))
    ///     .unwrap()
    ///     .icon(IconConfig::builder().size(64).unwrap().build())
    ///     .windows_ignore_rules([
    ///         IgnoreRule::builder()
    ///             .process_name("whatever")
    ///             .window_title(WindowTitleMatch::Missing)
    ///             .build(),
    ///     ])
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
            with = |rules: impl IntoIterator<Item = IgnoreRule>| IgnoreRules::new(rules),
        )]
        linux_ignore_rules: IgnoreRules,
        #[builder(
            default,
            with = |rules: impl IntoIterator<Item = IgnoreRule>| IgnoreRules::new(rules),
        )]
        macos_ignore_rules: IgnoreRules,
        #[builder(
            default,
            with = |rules: impl IntoIterator<Item = IgnoreRule>| IgnoreRules::new(rules),
        )]
        windows_ignore_rules: IgnoreRules,
    ) -> Self {
        Self {
            poll_interval,
            icon,
            icon_cache_capacity,
            linux_ignore_rules,
            macos_ignore_rules,
            windows_ignore_rules,
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
    fn config_default_ignore_rules_are_empty() {
        let config = FocusTrackerConfig::default();
        assert!(config.linux_ignore_rules.is_empty());
        assert!(config.macos_ignore_rules.is_empty());
        assert!(config.windows_ignore_rules.is_empty());
    }

    #[test]
    fn config_builder_per_platform_ignore_rules() {
        let config = FocusTrackerConfig::builder()
            .linux_ignore_rules([IgnoreRule::builder().process_name("firefox").build()])
            .macos_ignore_rules([IgnoreRule::builder().process_name("Firefox").build()])
            .windows_ignore_rules([
                IgnoreRule::builder().process_name("firefox.exe").build(),
                IgnoreRule::builder().process_name("chrome.exe").build(),
            ])
            .build();

        assert!(config.linux_ignore_rules.matches("firefox", None));
        assert!(!config.linux_ignore_rules.matches("firefox.exe", None));

        assert!(config.macos_ignore_rules.matches("Firefox", None));
        assert!(!config.macos_ignore_rules.matches("firefox", None));

        assert_eq!(config.windows_ignore_rules.len(), 2);
        assert!(config.windows_ignore_rules.matches("firefox.exe", None));
        assert!(config.windows_ignore_rules.matches("chrome.exe", None));
    }

    #[test]
    fn config_builder_supports_title_aware_rules() {
        use crate::WindowTitleMatch;

        // The motivating case: suppress "whatever" only when it has no
        // title; keep events for titled instances.
        let config = FocusTrackerConfig::builder()
            .windows_ignore_rules([IgnoreRule::builder()
                .process_name("whatever")
                .window_title(WindowTitleMatch::Missing)
                .build()])
            .build();

        assert!(config.windows_ignore_rules.matches("whatever", None));
        assert!(config.windows_ignore_rules.matches("whatever", Some("")));
        assert!(!config.windows_ignore_rules.matches("whatever", Some("Doc")));
        assert!(!config.windows_ignore_rules.matches("other", None));
    }

    #[test]
    fn config_current_platform_selector_matches_target() {
        let config = FocusTrackerConfig::builder()
            .linux_ignore_rules([IgnoreRule::builder().process_name("lin").build()])
            .macos_ignore_rules([IgnoreRule::builder().process_name("mac").build()])
            .windows_ignore_rules([IgnoreRule::builder().process_name("win").build()])
            .build();

        let current = config.ignore_rules_for_current_platform();
        #[cfg(target_os = "linux")]
        assert!(current.matches("lin", None));
        #[cfg(target_os = "macos")]
        assert!(current.matches("mac", None));
        #[cfg(target_os = "windows")]
        assert!(current.matches("win", None));
    }
}
