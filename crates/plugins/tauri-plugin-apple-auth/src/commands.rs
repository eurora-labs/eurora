use tauri::{AppHandle, Runtime, command};

use crate::models::{AppleSignInOutcome, SignInRequest};
use crate::{AppleAuthExt, Result};

#[command]
pub(crate) async fn sign_in_with_apple<R: Runtime>(
    app: AppHandle<R>,
    payload: SignInRequest,
) -> Result<AppleSignInOutcome> {
    app.apple_auth().sign_in_with_apple(payload).await
}
