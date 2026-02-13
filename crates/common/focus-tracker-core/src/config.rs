use crate::{FocusTrackerError, FocusTrackerResult};
use std::time::Duration;

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

impl IconConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_size(mut self, size: u32) -> FocusTrackerResult<Self> {
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
        self.size = Some(size);
        Ok(self)
    }

    pub fn get_size_or_default(&self) -> u32 {
        self.size.unwrap_or(128)
    }
}

#[derive(Debug, Clone)]
pub struct FocusTrackerConfig {
    pub poll_interval: Duration,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_icon_config(mut self, icon: IconConfig) -> Self {
        self.icon = icon;
        self
    }

    pub fn with_icon_size(mut self, size: u32) -> FocusTrackerResult<Self> {
        self.icon = self.icon.with_size(size)?;
        Ok(self)
    }

    pub fn with_poll_interval(mut self, interval: Duration) -> FocusTrackerResult<Self> {
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
        self.poll_interval = interval;
        Ok(self)
    }

    pub fn with_poll_interval_ms(self, ms: u64) -> FocusTrackerResult<Self> {
        self.with_poll_interval(Duration::from_millis(ms))
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
        let config = FocusTrackerConfig::new()
            .with_poll_interval_ms(250)
            .unwrap();
        assert_eq!(config.poll_interval, Duration::from_millis(250));
    }

    #[test]
    fn test_icon_config_builder() {
        let config = FocusTrackerConfig::new().with_icon_size(64).unwrap();
        assert_eq!(config.icon.size, Some(64));
    }

    #[test]
    fn test_icon_config_default_size() {
        let icon_config = IconConfig::new();
        assert_eq!(icon_config.get_size_or_default(), 128);
    }

    #[test]
    fn test_icon_config_with_size() {
        let icon_config = IconConfig::new().with_size(256).unwrap();
        assert_eq!(icon_config.size, Some(256));
        assert_eq!(icon_config.get_size_or_default(), 256);
    }

    #[test]
    fn test_zero_icon_size_errors() {
        let result = IconConfig::new().with_size(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_large_icon_size_errors() {
        let result = IconConfig::new().with_size(1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_with_poll_interval() {
        let config = FocusTrackerConfig::new()
            .with_poll_interval(Duration::from_millis(500))
            .unwrap();
        assert_eq!(config.poll_interval, Duration::from_millis(500));
    }

    #[test]
    fn test_zero_interval_errors() {
        let result = FocusTrackerConfig::new().with_poll_interval(Duration::from_millis(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_large_interval_errors() {
        let result = FocusTrackerConfig::new().with_poll_interval(Duration::from_secs(11));
        assert!(result.is_err());
    }
}
