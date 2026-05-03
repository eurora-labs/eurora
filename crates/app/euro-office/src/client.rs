//! Desktop-side helpers for talking to the Office add-in over the
//! Eurora bridge.
//!
//! The add-in registers as a non-PID client with
//! `app_kind = Some(`[`MICROSOFT_WORD_KIND`]`)`. Callers locate it via
//! [`euro_browser::BridgeService::find_clients_by_kind`] and then issue
//! `GET_ASSETS` / `GET_METADATA` requests like any other bridge client.

use euro_browser::BridgeService;

use crate::WordDocumentAsset;

/// Logical client identifier the Word add-in sends in its
/// `RegisterFrame`. Used by the desktop strategy to locate the add-in's
/// session pid via [`BridgeService::find_clients_by_kind`].
pub const MICROSOFT_WORD_KIND: &str = "microsoft-word";

/// Bridge action requesting the Word add-in's current document asset.
///
/// The add-in responds with a [`ResponseFrame`] whose `payload` is a
/// JSON-encoded [`WordDocumentAsset`] (no `NativeMessage` wrapper).
///
/// [`ResponseFrame`]: euro_browser::ResponseFrame
pub const ACTION_GET_ASSETS: &str = "GET_ASSETS";

/// Fetch the current [`WordDocumentAsset`] from the first registered
/// Word add-in.
///
/// Returns `None` when no `microsoft-word` client is connected, when
/// the bridge request fails or times out, when the response carries no
/// payload, or when the payload fails to deserialize. All of these are
/// soft-failure conditions for the calling strategy: an absent asset
/// just means "try again on the next collection tick".
///
/// First-client policy: if multiple Word documents are open and each
/// hosts its own add-in instance, this picks one arbitrarily. The OS
/// focus tracker can only tell us *Word* is focused, not *which*
/// document, so per-document correlation is left for a follow-up.
pub async fn fetch_word_asset(service: &BridgeService) -> Option<WordDocumentAsset> {
    let app_pid = *service.find_clients_by_kind(MICROSOFT_WORD_KIND).first()?;

    let response = match service.send_request(app_pid, ACTION_GET_ASSETS, None).await {
        Ok(response) => response,
        Err(err) => {
            tracing::debug!(
                "Word add-in (pid={app_pid}) {ACTION_GET_ASSETS} request failed: {err}"
            );
            return None;
        }
    };

    let payload = response.payload?;
    match serde_json::from_str::<WordDocumentAsset>(&payload) {
        Ok(asset) => Some(asset),
        Err(err) => {
            tracing::warn!("Word add-in returned malformed {ACTION_GET_ASSETS} payload: {err}");
            None
        }
    }
}
