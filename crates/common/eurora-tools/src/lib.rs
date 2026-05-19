//! Eurora's in-process tool-execution framework.
//!
//! This crate defines the types and traits that tie the LLM-facing
//! "one flat tool catalog" model to the routing realities of a real
//! desktop client: descriptors that the macro can emit and the server can
//! consume; `Origin` variants that name a specific tab, window, or ACP
//! session; a [`Catalog`] of [`Dispatcher`]s on the client; and a
//! [`RemoteToolBus`] abstraction the server-side agent loop uses to fire
//! a tool call across the chat WebSocket.
//!
//! The wire-side counterparts ([`thread_core::WireToolDescriptor`],
//! [`thread_core::ToolErrorWire`], etc.) live in `thread-core`; this
//! crate re-exports them for ergonomic adapter authoring.
//!
//! # Architecture
//!
//! - **[`ToolDescriptor`]** — framework-side, `&'static`-everywhere,
//!   embedded in macro-emitted descriptor tables. Converts to
//!   [`thread_core::WireToolDescriptor`] via
//!   [`ToolDescriptor::to_wire`].
//! - **[`Origin`]** — typed routing target; never crosses the
//!   WebSocket. The client snapshots one origin per active context at
//!   turn start and the dispatcher consumes it in [`IncomingCall`].
//! - **[`ToolError`]** — in-process error type with structured causes;
//!   converts (lossily for `Decode`/`Encode`/`Adapter`) to
//!   [`thread_core::ToolErrorWire`].
//! - **[`Dispatcher`]** — per-adapter trait the macro implements. The
//!   [`Catalog`] indexes dispatchers by descriptor name and is what
//!   `ChatBridge` looks tools up against on the client.
//! - **[`RemoteToolBus`]** — server-side bus the agent loop calls into
//!   when a tool's `ToolSource` is anything other than `ServerLocal`.
//! - **[`schema_of`]** — shared JSON-Schema cache backing the
//!   `input_schema` / `output_schema` accessors on `ToolDescriptor`.
//!
//! See `plan.md` for the end-to-end design.

mod bus;
mod descriptor;
mod dispatcher;
mod error;
mod origin;
mod schema;

#[doc(hidden)]
pub mod __private;

pub use bus::{IncomingCall, RemoteToolBus, RemoteToolBusLocal};
pub use descriptor::ToolDescriptor;
pub use dispatcher::{Catalog, Dispatcher};
pub use error::ToolError;
pub use origin::{AcpOrigin, BrowserOrigin, FocusedOrigin, Origin};
pub use schema::{SchemaFn, schema_of};

pub use eurora_tools_macros::{adapter, tool};

// Wire types are re-exported so adapter crates depend on `eurora-tools`
// alone and don't need a direct `thread-core` import.
pub use thread_core::{ToolErrorWire, ToolSource, WireActiveContext, WireToolDescriptor};
