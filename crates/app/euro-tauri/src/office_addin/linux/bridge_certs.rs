//! Linux fallback for the bridge CA trust install. Word doesn't run
//! natively here, so there's nothing to register and `TrustOutcome::Skipped`
//! is the truthful answer.

use std::path::Path;

use crate::office_addin::bridge_certs::{TrustAction, TrustOutcome};

pub fn trust_impl(_ca_path: &Path, _action: TrustAction) -> TrustOutcome {
    TrustOutcome::Skipped
}
