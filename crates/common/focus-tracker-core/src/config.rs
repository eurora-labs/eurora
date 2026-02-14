use bon::bon;

use crate::{FocusTrackerError, FocusTrackerResult};
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
}

impl Default for FocusTrackerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            icon: IconConfig::default(),
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
    ) -> Self {
        Self {
            poll_interval,
            icon,
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
    }

    #[test]
    fn config_builder_defaults() {
        let config = FocusTrackerConfig::builder().build();
        assert_eq!(config.poll_interval, Duration::from_millis(100));
        assert_eq!(config.icon.size, None);
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
}
