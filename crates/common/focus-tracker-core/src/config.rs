use std::time::Duration;

/// Configuration for icon processing behavior
#[derive(Debug, Clone)]
pub struct IconConfig {
    /// Target size for icons (width and height will be equal)
    /// Default: None (use platform default size)
    pub size: Option<u32>,

    /// The algorithm to use for icon scaling, supports Windows and Linux x11
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

impl IconConfig {
    /// Create a new icon configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the icon size (width and height will be equal)
    ///
    /// # Arguments
    /// * `size` - The icon size in pixels
    ///
    /// # Panics
    /// Panics if the size is zero or too large (> 512)
    pub fn with_size(mut self, size: u32) -> Self {
        self.validate_size(size);
        self.size = Some(size);
        self
    }

    /// Get the icon size, using a default if none is configured
    pub fn get_size_or_default(&self) -> u32 {
        self.size.unwrap_or(128) // Default to 128x128
    }

    /// Validate the icon size
    fn validate_size(&self, size: u32) {
        if size == 0 {
            panic!("Icon size cannot be zero");
        }
        if size > 512 {
            panic!("Icon size cannot be greater than 512 pixels");
        }
    }
}

/// Configuration for focus tracking behavior
#[derive(Debug, Clone)]
pub struct FocusTrackerConfig {
    /// Polling interval for focus change detection
    /// Default: 100ms
    pub poll_interval: Duration,
    /// Icon processing configuration
    /// Default: IconConfig::default()
    pub icon: IconConfig,
}

impl Default for FocusTrackerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            icon: IconConfig::default(),
        }
    }
}

impl FocusTrackerConfig {
    /// Create a new configuration with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the icon configuration
    ///
    /// # Arguments
    /// * `icon` - The icon configuration
    pub fn with_icon_config(mut self, icon: IconConfig) -> Self {
        self.icon = icon;
        self
    }

    /// Set the icon size (convenience method)
    ///
    /// # Arguments
    /// * `size` - The icon size in pixels
    ///
    /// # Panics
    /// Panics if the size is zero or too large (> 512)
    pub fn with_icon_size(mut self, size: u32) -> Self {
        self.icon = self.icon.with_size(size);
        self
    }

    /// Set the polling interval for focus change detection
    ///
    /// # Arguments
    /// * `interval` - The polling interval duration
    ///
    /// # Panics
    /// Panics if the interval is zero or too large (> 10 seconds)
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.validate_poll_interval(interval);
        self.poll_interval = interval;
        self
    }

    /// Set the polling interval in milliseconds
    ///
    /// # Arguments
    /// * `ms` - The polling interval in milliseconds
    ///
    /// # Panics
    /// Panics if the interval is zero or too large (> 10000ms)
    pub fn with_poll_interval_ms(self, ms: u64) -> Self {
        self.with_poll_interval(Duration::from_millis(ms))
    }

    /// Validate the polling interval
    fn validate_poll_interval(&self, interval: Duration) {
        if interval.is_zero() {
            panic!("Poll interval cannot be zero");
        }
        if interval > Duration::from_secs(10) {
            panic!("Poll interval cannot be greater than 10 seconds");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FocusTrackerConfig::default();
        assert_eq!(config.poll_interval, Duration::from_millis(100));
    }

    #[test]
    fn test_default_icon_config() {
        let config = FocusTrackerConfig::default();
        assert_eq!(config.icon.size, None);
    }

    #[test]
    fn test_builder_pattern() {
        let config = FocusTrackerConfig::new().with_poll_interval_ms(250);
        assert_eq!(config.poll_interval, Duration::from_millis(250));
    }

    #[test]
    fn test_icon_config_builder() {
        let config = FocusTrackerConfig::new().with_icon_size(64);
        assert_eq!(config.icon.size, Some(64));
    }

    #[test]
    fn test_icon_config_default_size() {
        let icon_config = IconConfig::new();
        assert_eq!(icon_config.get_size_or_default(), 128);
    }

    #[test]
    fn test_icon_config_with_size() {
        let icon_config = IconConfig::new().with_size(256);
        assert_eq!(icon_config.size, Some(256));
        assert_eq!(icon_config.get_size_or_default(), 256);
    }

    #[test]
    #[should_panic(expected = "Icon size cannot be zero")]
    fn test_zero_icon_size_panics() {
        IconConfig::new().with_size(0);
    }

    #[test]
    #[should_panic(expected = "Icon size cannot be greater than 512 pixels")]
    fn test_large_icon_size_panics() {
        IconConfig::new().with_size(1024);
    }

    #[test]
    fn test_with_poll_interval() {
        let config = FocusTrackerConfig::new().with_poll_interval(Duration::from_millis(500));
        assert_eq!(config.poll_interval, Duration::from_millis(500));
    }

    #[test]
    #[should_panic(expected = "Poll interval cannot be zero")]
    fn test_zero_interval_panics() {
        FocusTrackerConfig::new().with_poll_interval(Duration::from_millis(0));
    }

    #[test]
    #[should_panic(expected = "Poll interval cannot be greater than 10 seconds")]
    fn test_large_interval_panics() {
        FocusTrackerConfig::new().with_poll_interval(Duration::from_secs(11));
    }
}
