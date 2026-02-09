//! Helper functions for managing the Agent Chain API.
//!
//! This module is only relevant for Agent Chain developers, not for users.
//!
//! **Warning:** This module and its submodules are for internal use only.
//! Do not use them in your own code. We may change the API at any time with no warning.

mod beta;
mod deprecation;
mod internal;
mod path;

pub use beta::{AgentChainBetaWarning, BetaParams, warn_beta};
pub use deprecation::{
    AgentChainDeprecationWarning, AgentChainPendingDeprecationWarning, DeprecationParams,
    RenameParameterParams, handle_renamed_parameter, warn_deprecated,
};
pub use internal::is_caller_internal;
pub use path::{as_import_path, get_relative_path};
