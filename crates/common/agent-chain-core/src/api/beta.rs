use std::fmt;

use super::internal::is_caller_internal;

#[derive(Debug, Clone)]
pub struct AgentChainBetaWarning {
    message: String,
}

impl AgentChainBetaWarning {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

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

#[derive(Debug, Clone, Default)]
pub struct BetaParams {
    pub message: Option<String>,
    pub name: Option<String>,
    pub obj_type: Option<String>,
    pub addendum: Option<String>,
}

impl BetaParams {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_obj_type(mut self, obj_type: impl Into<String>) -> Self {
        self.obj_type = Some(obj_type.into());
        self
    }

    pub fn with_addendum(mut self, addendum: impl Into<String>) -> Self {
        self.addendum = Some(addendum.into());
        self
    }
}

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
