//! Named timeouts and backoffs for every Eurora transport path.
//!
//! Every duration that gates a network round-trip, a reconnect attempt,
//! a heartbeat cadence, or a stream drain lives here. The goal is one
//! place to read, one place to change, and the discipline that "magic
//! number in a `Duration::from_secs(_)`" inside a transport crate is a
//! bug to be fixed by promoting the literal into this module.
//!
//! Constants are grouped by transport stage so readers can scan a
//! lifecycle (handshake → steady-state → shutdown → drain) without
//! cross-referencing four crates.

use std::time::Duration;

// ─── Bridge WebSocket (desktop ⇄ native host / Office add-in) ──────────

/// Maximum time the desktop will wait for the first frame from a
/// freshly-connected bridge client before dropping the WebSocket.
///
/// The first frame is always a [`RegisterFrame`]; if it doesn't arrive
/// in this window the client is misbehaving (or the link is silently
/// broken) and the bridge tears the connection down so the slot can be
/// reused.
///
/// [`RegisterFrame`]: <https://docs.rs/euro-bridge-protocol>
pub const BRIDGE_REGISTER_TIMEOUT: Duration = Duration::from_secs(5);

/// Cadence of WebSocket pings from the bridge to each connected client.
///
/// Long enough that idle connections don't waste CPU on a busy machine;
/// short enough that a half-open TCP connection is detected before a
/// chat turn times out at [`CHAT_STREAM_TIMEOUT`].
pub const BRIDGE_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Default per-request RPC timeout for desktop-initiated calls down to
/// a bridge client. A specific call site may pass its own timeout via
/// the request envelope; this is the ceiling when none is supplied.
pub const BRIDGE_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Time the bridge gives in-flight WebSocket connections to drain when
/// the server is shutting down before the listener is forced closed.
pub const BRIDGE_SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

// ─── Native-messaging host (browser ⇄ desktop bridge) ──────────────────

/// Constant backoff between bridge reconnect attempts from the browser
/// native-messaging host. Doubling backoff would only delay surfacing
/// "desktop is down" to the user without reducing real load on a
/// localhost socket; constant is correct here.
pub const NATIVE_HOST_RECONNECT_BACKOFF: Duration = Duration::from_secs(2);

// ─── Chat stream (LLM turn ⇄ desktop ⇄ frontend) ───────────────────────

/// Wall-clock ceiling on a single chat turn — long enough for slow
/// thinking models and big tool round-trips, short enough that a stuck
/// upstream surfaces as a clean error rather than a hung UI.
pub const CHAT_STREAM_TIMEOUT: Duration = Duration::from_secs(300);

/// After a chat turn emits its terminal frame, in-flight tool dispatch
/// tasks get this much time to settle before the runtime stops awaiting
/// them. A stuck adapter cannot pin the turn beyond this window.
pub const CHAT_DISPATCH_DRAIN: Duration = Duration::from_secs(1);
