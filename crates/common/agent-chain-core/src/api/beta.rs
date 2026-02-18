//! Helper functions for marking parts of the Agent Chain API as beta.
//!
//! This module was loosely adapted from matplotlib's _api/deprecation.py module:
//! https://github.com/matplotlib/matplotlib/blob/main/lib/matplotlib/_api/deprecation.py
//!
//! **Warning:** This module is for internal use only. Do not use it in your own code.
//! We may change the API at any time with no warning.

use std::fmt;

use super::internal::is_caller_internal;

/// A warning type for beta features in Agent Chain.
#[derive(Debug, Clone)]
pub struct AgentChainBetaWarning {
    message: String,
}

impl AgentChainBetaWarning {
    /// Create a new beta warning with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Get the warning message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AgentChainBetaWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AgentChainBetaWarning {}

/// Parameters for configuring beta warnings.
#[derive(Debug, Clone, Default)]
pub struct BetaParams {
    /// Override the default beta message.
    pub message: Option<String>,
    /// The name of the beta object.
    pub name: Option<String>,
    /// The object type being marked as beta (e.g., "function", "class", "method").
    pub obj_type: Option<String>,
    /// Additional text appended directly to the final message.
    pub addendum: Option<String>,
}

impl BetaParams {
    /// Create new beta parameters with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    /// Set the custom message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the object type.
    pub fn with_obj_type(mut self, obj_type: impl Into<String>) -> Self {
        self.obj_type = Some(obj_type.into());
        self
    }

    /// Set the addendum.
    pub fn with_addendum(mut self, addendum: impl Into<String>) -> Self {
        self.addendum = Some(addendum.into());
        self
    }
}

/// Display a standardized beta warning.
///
/// # Arguments
///
/// * `params` - Parameters for the beta warning.
/// * `caller_module` - The module path of the caller (typically from `module_path!()` macro).
///
/// # Example
///
/// ```
/// use agent_chain_core::api::{warn_beta, BetaParams};
///
/// // Simple warning
/// warn_beta(BetaParams::new("my_function"), module_path!());
///
/// // With additional details
/// warn_beta(
///     BetaParams::new("MyClass")
///         .with_obj_type("class")
///         .with_addendum("Consider using StableClass instead."),
///     module_path!()
/// );
/// ```
pub fn warn_beta(params: BetaParams, caller_module: &str) {
    if is_caller_internal(caller_module) {
        return;
    }

    let message = if let Some(msg) = params.message {
        msg
    } else {
        let name = params.name.unwrap_or_else(|| "unknown".to_string());
        let mut msg = if let Some(obj_type) = params.obj_type {
            format!("The {} `{}`", obj_type, name)
        } else {
            format!("`{}`", name)
        };

        msg.push_str(" is in beta. It is actively being worked on, so the API may change.");

        if let Some(addendum) = params.addendum {
            msg.push(' ');
            msg.push_str(&addendum);
        }

        msg
    };

    let warning = AgentChainBetaWarning::new(message);
    tracing::warn!(target: "agent_chain_core::beta", %warning, "AgentChainBetaWarning");
}

/// Macro to mark a function or method as beta.
///
/// This macro emits a beta warning when the decorated item is called.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::beta;
///
/// #[beta]
/// fn experimental_feature() {
///     // Implementation
/// }
///
/// #[beta(name = "custom_name", addendum = "Use stable_feature instead.")]
/// fn another_beta_feature() {
///     // Implementation
/// }
/// ```
#[macro_export]
macro_rules! beta {
    ($name:expr) => {
        $crate::api::warn_beta($crate::api::BetaParams::new($name), module_path!())
    };
    ($name:expr, $($key:ident = $value:expr),+ $(,)?) => {{
        let mut params = $crate::api::BetaParams::new($name);
        $(
            params = $crate::api::beta!(@set params, $key, $value);
        )+
        $crate::api::warn_beta(params, module_path!())
    }};
    (@set $params:expr, message, $value:expr) => {
        $params.with_message($value)
    };
    (@set $params:expr, obj_type, $value:expr) => {
        $params.with_obj_type($value)
    };
    (@set $params:expr, addendum, $value:expr) => {
        $params.with_addendum($value)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beta_warning_creation() {
        let warning = AgentChainBetaWarning::new("Test warning");
        assert_eq!(warning.message(), "Test warning");
        assert_eq!(format!("{}", warning), "Test warning");
    }

    #[test]
    fn test_beta_params_builder() {
        let params = BetaParams::new("test_function")
            .with_obj_type("function")
            .with_addendum("Consider using other_function.");

        assert_eq!(params.name, Some("test_function".to_string()));
        assert_eq!(params.obj_type, Some("function".to_string()));
        assert_eq!(
            params.addendum,
            Some("Consider using other_function.".to_string())
        );
    }
}
