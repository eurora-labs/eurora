# Plan: Apple revoke + in-app account deletion

Two tightly-coupled features deferred from the original Sign in with Apple
plan (§6.4 and §6.5). Apple's revoke endpoint is only useful in the
context of account deletion (logout is per-device, revoke is "permanently
disconnect from this app"), so we plan them as one feature with revoke as
a prerequisite.

## Background

App Store Review Guideline **5.1.1(v)** requires that any app offering
Sign in with Apple also let the user delete their account *from inside the
app*, independent of Apple-side termination. Eurora's notifications
handler already covers Apple-initiated termination (`consent-revoked`,
`account-delete`); what's missing is a user-initiated path.

Apple's own spec additionally recommends calling
`https://appleid.apple.com/auth/revoke` with the user's stored refresh
token when they delete their account, so Apple-side state stays
consistent with ours.

## Current state worth knowing

- **All FKs to `users.id` already cascade.** `oauth_credentials`,
  `refresh_tokens`, `oauth_state`, `login_tokens`,
  `email_verification_tokens`, `token_usage`,
  `stripe_customer_provisioning`, `threads`, `messages`,
  `message_branches` all carry `ON DELETE CASCADE`.
  `stripe.customers.app_user_id` uses `ON DELETE SET NULL` —
  intentional, since billing history must outlive the user for refund /
  audit purposes. A single `DELETE FROM users WHERE id = $1` cleans up
  everything atomically.
- **Apple refresh tokens aren't stored today.**
  `apple_user_info_to_raw`
  (`crates/backend/be-auth-service/src/oauth/provider_ext.rs:340-355`)
  zeroes them out. To call revoke we have to start storing them.
- **`AppleTokenResponse` already deserialises `refresh_token`** but
  marks it `#[allow(dead_code)]`. Wiring it through is a small change,
  not a new parser.
- **Stripe**: the user's `stripe_customer_id` survives deletion with a
  `NULL` FK; if the user has an active subscription, they keep getting
  billed. We need a cancellation step before delete, otherwise we cause
  chargebacks.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│  Frontend (web + mobile + desktop): "Delete account" → confirm UI   │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼  DELETE /auth/account
┌─────────────────────────────────────────────────────────────────────┐
│  AuthService::delete_account(user_id)                               │
│    1. Re-auth gate: access token must be fresh (iat > now − 5min)   │
│    2. Cancel active Stripe subscriptions (best-effort + log)        │
│    3. Revoke Apple (if linked): decrypt refresh_token,              │
│       POST https://appleid.apple.com/auth/revoke (best-effort)      │
│    4. DELETE FROM users WHERE id = $1   (cascades to children)      │
│    5. Audit log: user_id, had_apple, had_subscription, outcomes     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Build order

### Step 1 — Persist Apple refresh tokens *(prerequisite for revoke)*

`crates/backend/be-auth-service/src/oauth/apple.rs`:

- `AppleUserInfo` gains `refresh_token: Option<SecretString>`. Apple's
  *access* token is genuinely useless (no userinfo endpoint), so we
  don't store it; only refresh.
- Remove `#[allow(dead_code)]` on `AppleTokenResponse.refresh_token`;
  populate `AppleUserInfo.refresh_token` in
  `exchange_code_with_redirect`.

`oauth/provider_ext.rs::apple_user_info_to_raw`:

- Threads `refresh_token` into `RawOAuthTokens.refresh_token`. The
  encryption boundary in `oauth_flow.rs::encrypt_token_bundle` already
  handles `Option<SecretString>`, so storage shape needs no change.
- Result: code-exchange paths (web / desktop / Android / iOS fallback)
  start writing an encrypted Apple refresh token to
  `oauth_credentials.refresh_token`. The native-iOS path still writes
  `None` — that's correct because `ASAuthorizationController` never
  exposes a refresh token, and the user can revoke from iOS Settings →
  Apple ID → Sign In with Apple if they want to disconnect without
  account deletion.

### Step 2 — `AppleOAuthClient::revoke_refresh_token`

`crates/backend/be-auth-service/src/oauth/apple.rs`:

```rust
pub async fn revoke_refresh_token(&self, refresh_token: &str) -> Result<(), OAuthError>
```

POSTs `application/x-www-form-urlencoded` to
`https://appleid.apple.com/auth/revoke` with:

- `client_id` = `service_id`
- `client_secret` = freshly minted JWT (reuse `mint_client_secret`)
- `token` = refresh_token
- `token_type_hint=refresh_token`

Error handling rules:

- **200** → success.
- **400 `invalid_grant`** → token already revoked; treat as success
  (end-state is identical, and Apple's notification webhook may have
  already revoked it).
- **400 other** → propagate as `OAuthError::RevokeFailed(String)`
  (new variant).
- **5xx / network** → propagate.

Tests with `wiremock`: success, invalid_grant-as-success,
server-error-propagates, mints fresh JWT each call.

### Step 3 — Stripe cancellation step

`crates/backend/be-payment-service` (or wherever the Stripe client
lives — see "Open questions" below):

- New method `cancel_active_subscriptions_for(user_id) ->
  Result<CancellationOutcome>`.
- Looks up `stripe.subscriptions` joined via
  `stripe.customers.app_user_id`, filters status in (`trialing`,
  `active`, `past_due`).
- For each, call Stripe's `Subscription::cancel` (immediate, not
  at-period-end — the user is leaving). Records what was cancelled.
- Best-effort: if Stripe is unreachable, log a `tracing::error!` with
  the subscription IDs but **proceed with deletion**. Refusing to
  delete because Stripe is down traps the user in their account; the
  error path produces a follow-up ticket for ops to manually reconcile.

This step belongs in the existing payment service, not in auth-service,
to keep the Stripe client out of the auth crate's dependency surface.
Auth calls into it via a trait or a thin function reference.

### Step 4 — `AuthService::delete_account`

`crates/backend/be-auth-service/src/account_deletion.rs` (new module):

```rust
pub async fn delete_account(&self, user_id: Uuid) -> AuthResult<DeleteAccountOutcome>
```

Steps:

1. Look up the user row (404 if missing — defends against
   double-clicks).
2. Look up `oauth_credentials` rows for the user. If an Apple one
   exists with a refresh token, decrypt and call
   `apple_oauth.revoke_refresh_token(...)`. Best-effort: log failure,
   don't block. Record `apple_revoke_attempted` +
   `apple_revoke_succeeded` in the outcome.
3. Call `payment_service.cancel_active_subscriptions_for(user_id)`
   (best-effort, see Step 3).
4. `DELETE FROM users WHERE id = $1`. Cascade does the rest.
5. Return `DeleteAccountOutcome { user_id, apple_revoked,
   subscriptions_canceled, residual_stripe_orphan }` for logging.

The `DeleteAccountOutcome` is logged at handler boundary (same pattern
as `AppleNotificationOutcome`), so structured fields are consistent.

### Step 5 — Re-auth freshness gate

Add `AccessClaims::require_fresh(max_age: Duration)` extractor — checks
`claims.iat > now - max_age`. New `AuthError::ReauthRequired` variant,
surfaces as **403** with `error_kind = "reauth_required"`.

The delete handler uses `AccessClaims::require_fresh(Duration::minutes(5))`.
The frontend's 403 handler dispatches `reauth_required` to a re-login
flow rather than the generic "session expired" path.

This isn't strictly required by Apple Review, but it's the difference
between "delete account" being a single click away from a left-open
laptop and being a properly-gated destructive operation. Standard
practice for Google / GitHub / Stripe.

### Step 6 — HTTP handler

`crates/backend/be-auth-service/src/handlers.rs`:

```rust
#[tracing::instrument(skip_all)]
pub async fn delete_account(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    claims: AccessClaims,  // with require_fresh applied
) -> AuthResult<(CookieJar, StatusCode)> {
    let user_id = claims.user_id()?;
    let outcome = state.auth.delete_account(user_id).await?;
    log_account_deletion(&outcome);
    let jar = cookies::clear_all(&state.cookies, jar);
    Ok((jar, StatusCode::NO_CONTENT))
}
```

Route: `DELETE /auth/account`. Using HTTP DELETE (not POST) for
semantic clarity — destructive, idempotent, no body. Most CSRF
middleware is fine with DELETE on a token-bound endpoint; verify
against the existing CORS config.

Response 204 + cleared cookies. The frontend immediately treats the
user as logged out.

### Step 7 — Frontend (web)

`apps/web/src/lib/services/auth-service.svelte.ts`:

- `deleteAccount()` — calls `DELETE /auth/account`, on success clears
  local state via `#clearLocal()` and resolves. On `reauth_required`
  403, throws a typed `ReauthRequiredError` the caller can dispatch.

`apps/web/src/routes/(auth)/settings/+page.svelte` (new — or add to
existing settings):

- "Danger Zone" section, separated by a divider, red-themed.
- "Delete account" button → modal:
  - Warning text: lists what will be deleted (chats, threads, billing
    relationship), what is irreversible.
  - **Confirmation field**: user must type their email exactly to
    enable the Delete button.
  - On submit, call `deleteAccount()`. Catch `ReauthRequiredError` →
    bounce to `/login?reason=reauth&next=/settings#delete-account`.
- After success: `goto('/')` with a one-time toast "Your account has
  been deleted."

### Step 8 — Frontend (mobile)

`apps/mobile/src/routes/settings/+page.svelte` (or wherever account
settings live — verify if one exists before scaffolding new):

- Same UX: red Danger Zone, "Delete account" button, confirmation
  requires typing email.
- New procedure `auth_delete_account` in
  `euro-mobile/src/procedures/auth_procedures.rs` — calls
  `auth_manager.delete_account()` (new `AuthManager::delete_account` +
  `AuthClient::delete_account` wrappers in `euro-auth`).
- On success: emit `AuthStateChanged { claims: None }`, clear local
  secret-store entries (`ACCESS_TOKEN_HANDLE`, `REFRESH_TOKEN_HANDLE`),
  route to `/login`.

### Step 9 — Frontend (desktop)

Same pattern as mobile but in `apps/desktop`. Lower priority — desktop
users can delete via web. Worth doing for parity but not blocking the
App Store submission gate.

### Step 10 — Tests

**Unit:**

- `revoke_refresh_token`: wiremock for success, invalid_grant →
  success, server error, mints fresh JWT each call.
- `delete_account`: with Apple link → revoke called; without → revoke
  skipped; revoke failure → deletion still proceeds; non-existent
  user → `AuthError::NotFound`.
- Re-auth freshness: claim `iat = now - 4min` accepted, `iat = now -
  6min` rejected.

**Integration:**

- End-to-end `DELETE /auth/account`: seed user with `oauth_credentials`
  + `refresh_tokens` + a thread + a message → DELETE → assert all
  child rows gone, `stripe.customers` survives with NULL
  `app_user_id`, audit log line written.
- 403 `reauth_required` for stale tokens.
- Confirm `clear_all` cookies appear on the 204 response.

**Manual:**

- Real Apple account → delete via web → verify
  `appleid.apple.com/account/manage` no longer lists Eurora under
  "Apps Using Apple ID".

---

## Decisions worth flagging

1. **Hard delete, no grace period.** Cleaner GDPR alignment, simpler
   code. Trade-off: a tap-mistake is irrecoverable. The
   email-confirmation gate + re-auth freshness keeps the footgun
   guarded.
2. **Revoke on account deletion only, not on logout.** Logout is
   per-device; revoke is permanent. Calling revoke on every logout
   would break the "I want to switch Apple accounts" UX. Document
   this in the `delete_account` doc comment so future-us doesn't
   second-guess.
3. **Best-effort Apple revoke + Stripe cancellation.** Both can fail
   without blocking the user's deletion. Refusing to delete because a
   third-party API is down traps the user; the alternative is a small
   operational follow-up when the third party returns. Audit log
   captures the failure for ops reconciliation.
4. **No "delete account" without re-auth in the last 5 minutes.** Yes,
   this means the user has to log in twice in some flows. That's
   correct for a destructive irreversible operation.
5. **Native-iOS users without an Apple refresh token still get a clean
   deletion.** Their `oauth_credentials.refresh_token` is `None`, the
   revoke step skips, and the user can manually disconnect via iOS
   Settings → Apple ID. This is acceptable per Apple's guidelines —
   they require an in-app *delete account* flow, not an in-app
   *revoke Apple authorization* flow.
6. **DELETE verb on `/auth/account`.** Idempotent destructive
   operation; standard REST. Other auth endpoints use POST because
   they have request bodies; this one doesn't.

---

## PR split

**PR A — Apple revoke + refresh-token persistence** (~1 day)

- Step 1: refresh-token storage
- Step 2: `revoke_refresh_token` + tests
- No new endpoints; pure backend foundation.

**PR B — Account deletion** (~3-4 days)

- Steps 3-6: Stripe cancel, `delete_account`, re-auth gate,
  handler + route
- Step 10: tests
- Backend complete; deletion callable via curl.

**PR C — Web UI** (~1-2 days)

- Step 7
- Account-deletion settings page + confirmation modal + reauth bounce
- App-Store-Review-ready from web.

**PR D — Mobile + desktop UI** (~2 days)

- Steps 8 & 9
- Procedure, client methods, settings page on both apps.

PR A is blocking for PR B. PR B is blocking for PRs C and D. C and D
are parallel.

---

## Open questions before coding

1. **Subscription cancellation contract**: is the existing payment
   service's Stripe client async-API-ready, or does it only handle
   inbound webhooks? If it's webhook-only, Step 3 needs a Stripe
   client extension first. Worth a 10-minute audit of
   `be-payment-service` before writing the account-deletion module.
2. **Existing settings pages**: are there account-settings pages
   already on web / mobile / desktop that we should extend, or do
   we need to create them from scratch? Affects PR C/D scope.
3. **Re-auth UX for OAuth users**: if a user signed in with Apple and
   their access token is stale, the bounce-to-reauth flow should send
   them back through Apple, not through email+password (which they
   don't have). The login page already does provider dispatch — verify
   that `?reason=reauth&next=/settings#delete-account` survives the
   OAuth round-trip.
