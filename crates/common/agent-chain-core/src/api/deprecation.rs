//! Helper functions for deprecating parts of the Agent Chain API.
//!
//! This module was adapted from matplotlib's _api/deprecation.py module:
//! https://github.com/matplotlib/matplotlib/blob/main/lib/matplotlib/_api/deprecation.py
//!
//! **Warning:** This module is for internal use only. Do not use it in your own code.
//! We may change the API at any time with no warning.

use std::fmt;

use super::internal::is_caller_internal;

/// A warning type for deprecated features in Agent Chain.
#[derive(Debug, Clone)]
pub struct AgentChainDeprecationWarning {
    message: String,
}

impl AgentChainDeprecationWarning {
    /// Create a new deprecation warning with the given message.
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

impl fmt::Display for AgentChainDeprecationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AgentChainDeprecationWarning {}

/// A warning type for pending deprecations in Agent Chain.
#[derive(Debug, Clone)]
pub struct AgentChainPendingDeprecationWarning {
    message: String,
}

impl AgentChainPendingDeprecationWarning {
    /// Create a new pending deprecation warning with the given message.
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

impl fmt::Display for AgentChainPendingDeprecationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AgentChainPendingDeprecationWarning {}

/// Parameters for configuring deprecation warnings.
#[derive(Debug, Clone, Default)]
pub struct DeprecationParams {
    /// The release at which this API became deprecated.
    pub since: String,
    /// Override the default deprecation message.
    pub message: Option<String>,
    /// The name of the deprecated object.
    pub name: Option<String>,
    /// An alternative API that the user may use in place of the deprecated API.
    pub alternative: Option<String>,
    /// An alternative import path that the user may use instead.
    pub alternative_import: Option<String>,
    /// If `true`, uses a pending deprecation warning instead.
    pub pending: bool,
    /// The object type being deprecated (e.g., "function", "class", "method").
    pub obj_type: Option<String>,
    /// Additional text appended directly to the final message.
    pub addendum: Option<String>,
    /// The expected removal version.
    pub removal: Option<String>,
    /// The package of the deprecated object.
    pub package: Option<String>,
}

impl DeprecationParams {
    /// Create new deprecation parameters with the version when deprecation started.
    pub fn new(since: impl Into<String>) -> Self {
        Self {
            since: since.into(),
            ..Default::default()
        }
    }

    /// Set the name of the deprecated item.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set a custom deprecation message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the alternative to use instead of the deprecated item.
    pub fn with_alternative(mut self, alternative: impl Into<String>) -> Self {
        self.alternative = Some(alternative.into());
        self
    }

    /// Set the alternative import path.
    pub fn with_alternative_import(mut self, alternative_import: impl Into<String>) -> Self {
        self.alternative_import = Some(alternative_import.into());
        self
    }

    /// Mark this as a pending deprecation.
    pub fn with_pending(mut self, pending: bool) -> Self {
        self.pending = pending;
        self
    }

    /// Set the object type.
    pub fn with_obj_type(mut self, obj_type: impl Into<String>) -> Self {
        self.obj_type = Some(obj_type.into());
        self
    }

    /// Set the addendum text.
    pub fn with_addendum(mut self, addendum: impl Into<String>) -> Self {
        self.addendum = Some(addendum.into());
        self
    }

    /// Set the expected removal version.
    pub fn with_removal(mut self, removal: impl Into<String>) -> Self {
        self.removal = Some(removal.into());
        self
    }

    /// Set the package name.
    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    /// Validate the deprecation parameters.
    ///
    /// Returns an error if the parameters are invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.pending && self.removal.is_some() {
            return Err("A pending deprecation cannot have a scheduled removal".to_string());
        }
        // Non-pending deprecations must have a removal version specified
        // This matches Python's NotImplementedError behavior
        if !self.pending && self.removal.is_none() && self.message.is_none() {
            return Err(
                "Need to determine which default deprecation schedule to use. \
                 Non-pending deprecations must specify a removal version."
                    .to_string(),
            );
        }
        if self.alternative.is_some() && self.alternative_import.is_some() {
            return Err("Cannot specify both alternative and alternative_import".to_string());
        }
        if let Some(ref alt_import) = self.alternative_import
            && !alt_import.contains("::")
        {
            return Err(format!(
                "alternative_import must be a fully qualified module path. Got {}",
                alt_import
            ));
        }
        Ok(())
    }
}

/// Parameters for renaming a deprecated parameter.
#[derive(Debug, Clone)]
pub struct RenameParameterParams {
    /// The version in which the parameter was renamed.
    pub since: String,
    /// The version in which the old parameter will be removed.
    pub removal: String,
    /// The old parameter name.
    pub old: String,
    /// The new parameter name.
    pub new: String,
}

impl RenameParameterParams {
    /// Create new rename parameter params.
    pub fn new(
        since: impl Into<String>,
        removal: impl Into<String>,
        old: impl Into<String>,
        new: impl Into<String>,
    ) -> Self {
        Self {
            since: since.into(),
            removal: removal.into(),
            old: old.into(),
            new: new.into(),
        }
    }
}

/// Check if an old parameter name was used and emit a deprecation warning.
///
/// This function is used to handle parameter renaming with deprecation warnings.
/// It checks if the old parameter name is present in the provided parameters,
/// and if so, emits a deprecation warning and returns the value that should be used.
///
/// # Arguments
///
/// * `params` - The rename parameter configuration.
/// * `old_value` - The value passed with the old parameter name (if any).
/// * `new_value` - The value passed with the new parameter name (if any).
/// * `func_name` - The name of the function for the warning message.
/// * `caller_module` - The module path of the caller.
///
/// # Returns
///
/// * `Ok(value)` - The value to use (old_value takes precedence if both are provided).
/// * `Err(message)` - If both old and new parameters were provided.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::api::{handle_renamed_parameter, RenameParameterParams};
///
/// fn my_function(new_param: Option<String>, old_param: Option<String>) -> Result<(), String> {
///     let params = RenameParameterParams::new("0.1.0", "0.2.0", "old_param", "new_param");
///     let value = handle_renamed_parameter(
///         &params,
///         old_param,
///         new_param,
///         "my_function",
///         module_path!()
///     )?;
///     // Use `value` which is the resolved parameter value
///     Ok(())
/// }
/// ```
pub fn handle_renamed_parameter<T>(
    params: &RenameParameterParams,
    old_value: Option<T>,
    new_value: Option<T>,
    func_name: &str,
    caller_module: &str,
) -> Result<Option<T>, String> {
    match (old_value, new_value) {
        (Some(_), Some(_)) => Err(format!(
            "{}() got multiple values for argument '{}'",
            func_name, params.new
        )),
        (Some(old), None) => {
            // Emit deprecation warning for using old parameter
            warn_deprecated(
                DeprecationParams::new(&params.since)
                    .with_message(format!(
                        "The parameter `{}` of `{}` was deprecated in {} and will be removed in {}. Use `{}` instead.",
                        params.old, func_name, params.since, params.removal, params.new
                    ))
                    .with_removal(&params.removal),
                caller_module,
            );
            Ok(Some(old))
        }
        (None, new) => Ok(new),
    }
}

/// Display a standardized deprecation warning.
///
/// # Arguments
///
/// * `params` - Parameters for the deprecation warning.
/// * `caller_module` - The module path of the caller (typically from `module_path!()` macro).
///
/// # Example
///
/// ```
/// use agent_chain_core::api::{warn_deprecated, DeprecationParams};
///
/// // Simple deprecation warning
/// warn_deprecated(
///     DeprecationParams::new("0.1.0")
///         .with_name("old_function")
///         .with_removal("0.2.0"),
///     module_path!()
/// );
///
/// // With alternative
/// warn_deprecated(
///     DeprecationParams::new("0.1.0")
///         .with_name("OldClass")
///         .with_obj_type("class")
///         .with_alternative("NewClass")
///         .with_removal("0.2.0"),
///     module_path!()
/// );
/// ```
pub fn warn_deprecated(params: DeprecationParams, caller_module: &str) {
    // Skip if caller is internal
    if is_caller_internal(caller_module) {
        return;
    }

    // Validate parameters
    if let Err(err) = params.validate() {
        tracing::error!(target: "agent_chain_core::deprecation", %err, "Invalid deprecation parameters");
        return;
    }

    let message = if let Some(msg) = params.message {
        msg
    } else {
        let name = params.name.unwrap_or_else(|| "unknown".to_string());
        let package = params.package.unwrap_or_else(|| "agent-chain".to_string());

        let mut msg = if let Some(ref obj_type) = params.obj_type {
            format!("The {} `{}`", obj_type, name)
        } else {
            format!("`{}`", name)
        };

        if params.pending {
            msg.push_str(" will be deprecated in a future version");
        } else {
            msg.push_str(&format!(" was deprecated in {} {}", package, params.since));

            if let Some(ref removal) = params.removal {
                msg.push_str(&format!(" and will be removed in {}", removal));
            }
        }

        if let Some(ref alternative_import) = params.alternative_import {
            let alt_package = alternative_import
                .split("::")
                .next()
                .unwrap_or(alternative_import)
                .replace('_', "-");

            if alt_package == package {
                msg.push_str(&format!(". Use {} instead.", alternative_import));
            } else {
                let parts: Vec<&str> = alternative_import.rsplitn(2, "::").collect();
                if parts.len() == 2 {
                    let alt_name = parts[0];
                    let alt_module = parts[1];
                    msg.push_str(&format!(
                        ". An updated version of the {} exists in the {} package and should be used instead. \
                         To use it add `{}` to your dependencies and import as `use {}::{};`.",
                        params.obj_type.as_deref().unwrap_or("item"),
                        alt_package,
                        alt_package,
                        alt_module,
                        alt_name
                    ));
                }
            }
        } else if let Some(ref alternative) = params.alternative {
            msg.push_str(&format!(". Use {} instead.", alternative));
        }

        if let Some(ref addendum) = params.addendum {
            msg.push(' ');
            msg.push_str(addendum);
        }

        msg
    };

    if params.pending {
        let warning = AgentChainPendingDeprecationWarning::new(message);
        tracing::warn!(target: "agent_chain_core::deprecation", %warning, "AgentChainPendingDeprecationWarning");
    } else {
        let warning = AgentChainDeprecationWarning::new(message);
        tracing::warn!(target: "agent_chain_core::deprecation", %warning, "AgentChainDeprecationWarning");
    }
}

/// Macro for handling renamed parameters with deprecation warnings.
///
/// This macro simplifies the common pattern of renaming a function parameter
/// while maintaining backward compatibility.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::renamed_parameter;
///
/// fn my_function(new_param: Option<String>, old_param: Option<String>) -> Result<String, String> {
///     let value = renamed_parameter!(
///         since = "0.1.0",
///         removal = "0.2.0",
///         old = "old_param" => old_param,
///         new = "new_param" => new_param,
///         func = "my_function"
///     )?;
///     Ok(value.unwrap_or_default())
/// }
/// ```
#[macro_export]
macro_rules! renamed_parameter {
    (
        since = $since:expr,
        removal = $removal:expr,
        old = $old_name:expr => $old_value:expr,
        new = $new_name:expr => $new_value:expr,
        func = $func_name:expr
    ) => {{
        let params =
            $crate::api::RenameParameterParams::new($since, $removal, $old_name, $new_name);
        $crate::api::handle_renamed_parameter(
            &params,
            $old_value,
            $new_value,
            $func_name,
            module_path!(),
        )
    }};
}

/// Macro to emit a deprecation warning.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::deprecated;
///
/// // Simple deprecation
/// deprecated!("0.1.0", "old_function", removal = "0.2.0");
///
/// // With alternative
/// deprecated!("0.1.0", "OldClass",
///     obj_type = "class",
///     alternative = "NewClass",
///     removal = "0.2.0"
/// );
/// ```
#[macro_export]
macro_rules! deprecated {
    ($since:expr, $name:expr $(, $key:ident = $value:expr)* $(,)?) => {{
        let mut params = $crate::api::DeprecationParams::new($since).with_name($name);
        $(
            params = $crate::deprecated!(@set params, $key, $value);
        )*
        $crate::api::warn_deprecated(params, module_path!())
    }};
    (@set $params:expr, message, $value:expr) => {
        $params.with_message($value)
    };
    (@set $params:expr, alternative, $value:expr) => {
        $params.with_alternative($value)
    };
    (@set $params:expr, alternative_import, $value:expr) => {
        $params.with_alternative_import($value)
    };
    (@set $params:expr, pending, $value:expr) => {
        $params.with_pending($value)
    };
    (@set $params:expr, obj_type, $value:expr) => {
        $params.with_obj_type($value)
    };
    (@set $params:expr, addendum, $value:expr) => {
        $params.with_addendum($value)
    };
    (@set $params:expr, removal, $value:expr) => {
        $params.with_removal($value)
    };
    (@set $params:expr, package, $value:expr) => {
        $params.with_package($value)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deprecation_warning_creation() {
        let warning = AgentChainDeprecationWarning::new("Test warning");
        assert_eq!(warning.message(), "Test warning");
        assert_eq!(format!("{}", warning), "Test warning");
    }

    #[test]
    fn test_pending_deprecation_warning_creation() {
        let warning = AgentChainPendingDeprecationWarning::new("Test pending warning");
        assert_eq!(warning.message(), "Test pending warning");
        assert_eq!(format!("{}", warning), "Test pending warning");
    }

    #[test]
    fn test_deprecation_params_builder() {
        let params = DeprecationParams::new("0.1.0")
            .with_name("test_function")
            .with_obj_type("function")
            .with_alternative("new_function")
            .with_removal("0.2.0");

        assert_eq!(params.since, "0.1.0");
        assert_eq!(params.name, Some("test_function".to_string()));
        assert_eq!(params.obj_type, Some("function".to_string()));
        assert_eq!(params.alternative, Some("new_function".to_string()));
        assert_eq!(params.removal, Some("0.2.0".to_string()));
    }

    #[test]
    fn test_deprecation_params_validation() {
        // Valid params with removal
        let params = DeprecationParams::new("0.1.0")
            .with_name("test")
            .with_removal("0.2.0");
        assert!(params.validate().is_ok());

        // Valid pending params without removal
        let params = DeprecationParams::new("0.1.0")
            .with_name("test")
            .with_pending(true);
        assert!(params.validate().is_ok());

        // Valid params with custom message (doesn't require removal)
        let params = DeprecationParams::new("0.1.0")
            .with_name("test")
            .with_message("Custom deprecation message");
        assert!(params.validate().is_ok());

        // Pending with removal is invalid
        let params = DeprecationParams::new("0.1.0")
            .with_pending(true)
            .with_removal("0.2.0");
        assert!(params.validate().is_err());

        // Non-pending without removal is invalid (matches Python's NotImplementedError)
        let params = DeprecationParams::new("0.1.0").with_name("test");
        assert!(params.validate().is_err());

        // Both alternative and alternative_import is invalid
        let params = DeprecationParams::new("0.1.0")
            .with_alternative("new_thing")
            .with_alternative_import("some::path::NewThing")
            .with_removal("0.2.0");
        assert!(params.validate().is_err());

        // alternative_import without :: is invalid
        let params = DeprecationParams::new("0.1.0")
            .with_alternative_import("InvalidPath")
            .with_removal("0.2.0");
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_rename_parameter_params() {
        let params = RenameParameterParams::new("0.1.0", "0.2.0", "old_name", "new_name");
        assert_eq!(params.since, "0.1.0");
        assert_eq!(params.removal, "0.2.0");
        assert_eq!(params.old, "old_name");
        assert_eq!(params.new, "new_name");
    }

    #[test]
    fn test_handle_renamed_parameter_new_only() {
        let params = RenameParameterParams::new("0.1.0", "0.2.0", "old_param", "new_param");

        // Only new parameter provided - should return the new value
        let result = handle_renamed_parameter(
            &params,
            None::<String>,
            Some("new_value".to_string()),
            "test_func",
            "external_crate::module",
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("new_value".to_string()));
    }

    #[test]
    fn test_handle_renamed_parameter_old_only() {
        let params = RenameParameterParams::new("0.1.0", "0.2.0", "old_param", "new_param");

        // Only old parameter provided - should return the old value (with warning)
        let result = handle_renamed_parameter(
            &params,
            Some("old_value".to_string()),
            None,
            "test_func",
            "external_crate::module",
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("old_value".to_string()));
    }

    #[test]
    fn test_handle_renamed_parameter_both_provided() {
        let params = RenameParameterParams::new("0.1.0", "0.2.0", "old_param", "new_param");

        // Both parameters provided - should return error
        let result = handle_renamed_parameter(
            &params,
            Some("old_value".to_string()),
            Some("new_value".to_string()),
            "test_func",
            "external_crate::module",
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("got multiple values for argument")
        );
    }

    #[test]
    fn test_handle_renamed_parameter_none() {
        let params = RenameParameterParams::new("0.1.0", "0.2.0", "old_param", "new_param");

        // Neither parameter provided - should return None
        let result = handle_renamed_parameter(
            &params,
            None::<String>,
            None,
            "test_func",
            "external_crate::module",
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }
}
