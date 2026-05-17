# Unified Tool-Execution Architecture

## Goal

Generalize Eurora's chat path so that the server-side LLM can invoke tools that
execute in four distinct places — server-local, client-local, client-routed
through the bridge, and piped through an ACP agent — using a single uniform
protocol on the existing chat WebSocket. The model must see one flat tool
catalog; where each tool runs is a routing concern hidden from it.

This document is the design plan for that rewrite. It assumes the existing
shapes in `be-thread-service` (the agent loop, `ChatServerMessage` /
`ChatClientMessage`, the WS at `/threads/{id}/chat`) and `euro-bridge`
(`Frame`, registered clients, request/response correlation) as the starting
point.

## Execution sites

| Site                                | Examples                                                                 | Where it runs                                                | Who invokes it                                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| **Server-local**                    | Firecrawl sidecar, RAG over user's cloud activity, `describe_image_tool` | Backend process                                              | `agent_loop.rs` directly, via existing `Arc<dyn BaseTool>`                                                  |
| **Client-local**                    | Screenshot, focus state, active app, local file read                     | Tauri desktop app                                            | Server emits `ToolRequest` over chat WS; client's `ClientToolRegistry` dispatches                           |
| **Client-routed**                   | YouTube channel metadata, browser DOM scrape, Word document selection    | Browser extension / Office add-in, reached via `euro-bridge` | Server emits `ToolRequest`; client routes through `BridgeAdapter` → `euro-bridge` → registered client       |
| **ACP-piped**                       | OpenCode-driven coding tasks, external agent subagents                   | User-installed ACP agent subprocess (e.g. `opencode acp`)    | Server emits `ToolRequest`; client routes through `AcpClient` to the spawned subprocess over stdio JSON-RPC |
| **MCP-routed** (additive, post-MVP) | Any user-installed MCP server                                            | Local stdio process on the user's machine                    | Server emits `ToolRequest`; client routes through `McpRegistry` to the addressed MCP server                 |

The first three are the v1 surface. ACP-piped lands once the protocol is
stable. MCP-routed is the open-ecosystem add-on for v1.x.

## Topology

```
┌──────────────────────────── be-monolith (backend) ──────────────────────────┐
│                                                                              │
│   be-thread-service                                                          │
│   ┌──────────────────────────────────────────────────────────────────────┐   │
│   │  agent_loop.rs ── streams LLM ──► ChatServerMessage::Chunk           │   │
│   │      │                                                               │   │
│   │      ▼                                                               │   │
│   │  ToolDispatcher ──┬─► ServerLocal(Arc<dyn BaseTool>) ── invoke       │   │
│   │                   │                                                  │   │
│   │                   └─► Remote(call_id, descriptor) ── RemoteToolBus   │   │
│   │                                                          │           │   │
│   │                                                          │ pending   │   │
│   │                                                          │ oneshot   │   │
│   └──────────────────────────────────────────────────────────┼───────────┘   │
│                                                              │               │
└──────────────────────────────────────────────────────────────┼───────────────┘
                                                               │
                       chat WebSocket  ws://.../threads/{id}/chat
                                                               │
                       outbound: ChatServerMessage             │
                       inbound:  ChatClientMessage             │
                                                               │
┌──────────────────────────────────────────────────────────────┼───────────────┐
│                                                              ▼               │
│   euro-tauri (desktop app)                                                   │
│   ┌──────────────────────────────────────────────────────────────────────┐   │
│   │  ChatBridge                                                          │   │
│   │    receives ToolRequest, checks policy, routes by descriptor.source: │   │
│   │                                                                      │   │
│   │      ├─► ClientToolRegistry  (native Tauri commands)                 │   │
│   │      │      screenshot, focus, file read, etc.                       │   │
│   │      │                                                               │   │
│   │      ├─► BridgeAdapter       ──► euro-bridge                         │   │
│   │      │      (RequestFrame action+payload over loopback WS)           │   │
│   │      │                                                               │   │
│   │      ├─► AcpClient           ──► agent-client-protocol crate         │   │
│   │      │      spawns user-selected ACP agent (e.g. opencode acp)       │   │
│   │      │                                                               │   │
│   │      └─► McpRegistry         ──► user-installed MCP servers          │   │
│   │             (post-MVP)                                               │   │
│   └──────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────┘
        │                       │                    │                   │
        │ native Tauri cmd      │ ws://localhost:1431/bridge             │
        │ in-process            │ (existing euro-bridge protocol)        │
        ▼                       ▼                    ▼                   ▼
   ┌─────────┐         ┌────────────────┐    ┌────────────────┐  ┌──────────────┐
   │ native  │         │ euro-browser   │    │ OpenCode acp   │  │ user MCP     │
   │ handler │         │ native-msg     │    │ subprocess     │  │ servers      │
   │         │         │ host           │    │ (stdio JSON-RPC│  │ (stdio)      │
   └─────────┘         └───────┬────────┘    └────────────────┘  └──────────────┘
                               │ native messaging
                               ▼
                       ┌────────────────┐
                       │ apps/browser   │
                       │ (extension)    │
                       └────────────────┘
```

## Wire protocol — extensions to chat-stream

Today, `thread-core::ChatServerMessage` and `ChatClientMessage` carry only chat
turn frames (`Send`, `Regenerate`, `Cancel`, `Chunk`, `Final`, `Error`, `ConfirmedHumanMessage`).
The rewrite adds tool-routing frames so the chat WS becomes the single channel
that multiplexes streaming output **and** remote-tool RPC.

### New `ChatServerMessage` variants (server → client)

```text
ToolRequest      { call_id, descriptor, arguments }
ToolCancel       { call_id }
PermissionRequest{ call_id, descriptor, arguments, rationale }
```

`descriptor` is the same `ToolDescriptor` shape published by the client in the
`Hello` frame (see below). Echoing it back lets the client route without
re-resolving the tool name.

### New `ChatClientMessage` variants (client → server)

```text
Hello            { client_capabilities: Vec<ToolDescriptor>, policy_summary }
ToolResponse     { call_id, result: Ok(payload) | Err(code, message, details) }
ToolProgress     { call_id, payload }
PermissionResponse{ call_id, allow }
```

`Hello` is the new mandatory first frame on the chat WS, ahead of any
`Send` / `Regenerate`. It lets the client declare which tools it is willing
to serve this turn (filtered by user policy) so the backend can build the
LLM's tool list correctly.

### `ToolDescriptor`

```rust
struct ToolDescriptor {
    name: String,                 // namespaced, e.g. "browser.youtube.get_channel_metadata"
    description: String,
    input_schema: serde_json::Value,   // JSON Schema, fed to the LLM SDK
    output_schema: Option<serde_json::Value>,
    timeout_ms: u32,                   // server-enforced
    requires_user_approval: bool,
    source: ToolSource,
}

enum ToolSource {
    ServerLocal,
    ClientLocal,
    Bridge { app_kind: String },        // e.g. "browser", "microsoft-word"
    Acp    { session_id: String },
    Mcp    { server_id: String, tool_name: String }, // raw MCP tool name lives here
}
```

The descriptor is a _contract_ between server and client. The server uses
`name`, `description`, `input_schema` for the LLM. The client uses `source`
to route the inbound `ToolRequest`. The `source` round-trips: server echoes
the descriptor when sending `ToolRequest` so the client doesn't need to look
up by name.

### Why these specific frames

- **`Hello` for capabilities.** Discovery vs. declaration. We declare,
  because the user's policy is the source of truth — the client knows which
  tools the user has authorized this session and should be the one to
  announce them. The server never probes.
- **`ToolRequest` / `ToolResponse` with `call_id`.** Same pattern as
  `euro-bridge`'s `RequestFrame` / `ResponseFrame`. Correlation by id,
  oneshot resolution on the server side.
- **`ToolProgress` correlated by `call_id`.** Distinct from `EventFrame`
  (which is unsolicited push) so long-running tools — ACP `session/update`
  passthrough, Firecrawl scrapes — can stream updates without being
  confused with general events.
- **`PermissionRequest` / `PermissionResponse`.** Even though `Hello`
  declares the catalog, per-call approval is sometimes required (e.g. ACP
  agent asking to write a file). This is the escape hatch.
- **`ToolCancel`.** When the user cancels the turn or the server's tool
  budget is exhausted, in-flight remote calls have to be aborted. Mirrors
  the existing `CancellationToken` semantics on the server side.

## Server-side design

### Tool catalog

A new `ToolDescriptor` registry lives in (or adjacent to) `be-thread-service`.
Server-local tools register themselves with `ToolSource::ServerLocal` and the
existing `Arc<dyn BaseTool>` plumbing. Client-side sources arrive on the
`Hello` frame at the start of each chat turn and are merged into the
per-turn catalog.

### Dispatcher

`agent_loop.rs`'s `execute_tool_calls` is replaced by a `ToolDispatcher` that
branches on `ToolSource`:

```text
match descriptor.source {
    ServerLocal => existing path: tool.invoke_tool_call(call).await
    _           => RemoteToolBus::call(call_id, descriptor, args).await
}
```

`RemoteToolBus` is the analog of `euro-bridge`'s pending-request map, scoped
to a single chat connection:

- Allocate `call_id`, insert `oneshot::Sender<ToolResult>` in a
  `DashMap<u64, _>` on the connection state.
- Send `ToolServerMessage::ToolRequest { call_id, descriptor, arguments }`
  on the outbound WS channel.
- `tokio::select!` between the oneshot, the chat-level `CancellationToken`,
  and a per-tool timeout from the descriptor.
- On `ToolResponse` arrival, the WS reader task removes the entry and
  fulfills the oneshot.
- On cancel/timeout, emit `ToolCancel { call_id }` to the client.

### Persistence

Tool calls and their results are persisted as `ToolMessage` rows in the
existing thread store, regardless of execution site. The `agent_loop`'s
finalize path already does this for server-local tools; extend it to also
persist remote tool results from `ToolResponse` frames. Source-of-record
for transcripts stays on the backend.

### Files touched

- `thread-core` — extend `ChatServerMessage` / `ChatClientMessage` with the
  variants above; add `ToolDescriptor` and `ToolSource`.
- `be-thread-service/src/agent_loop.rs` — replace `execute_tool_calls` with
  source-aware dispatch.
- `be-thread-service/src/handlers/chat.rs` — accept `Hello` ahead of the
  first command frame; build per-turn tool catalog; thread `RemoteToolBus`
  into the spawned loop.
- New `be-thread-service/src/remote_tool_bus.rs` — pending-call bookkeeping,
  cancel/timeout handling, frame send.
- New `be-thread-service/src/tool_catalog.rs` — descriptor merging,
  validation of client-declared schemas.

## Client-side design

### `ChatBridge` (new module under `euro-tauri`)

Single owner of the chat WebSocket. Responsibilities:

- On connect: build `Hello` from the `ClientToolRegistry`, `BridgeAdapter`
  (which knows which `app_kind`s are currently registered with
  `euro-bridge`), `AcpClient` (if an agent is attached), and `McpRegistry`
  (if any servers are configured). Send `Hello`. Then send the user's
  `Send` / `Regenerate`.
- On inbound `Chunk` / `Final` / `Error` / `ConfirmedHumanMessage`: forward
  to the existing chat UI state.
- On inbound `ToolRequest`: look up descriptor's `source`, dispatch to the
  right subsystem, await result, send `ToolResponse`. For long-running
  calls, also emit `ToolProgress` frames as updates arrive.
- On inbound `ToolCancel`: tell the dispatched subsystem to abort.
- On inbound `PermissionRequest`: show native dialog, send
  `PermissionResponse`.

### `ClientToolRegistry`

Static table of `name → ClientTool` implementations for tools that run
inside the Tauri process: screenshot, focus state, active app, file reads,
activity timeline queries. Each has a typed handler.

### `BridgeAdapter`

Translates `ToolRequest { source: Bridge { app_kind }, ... }` into
`BridgeService::send_request(app_pid, action, payload)`:

- Resolve `app_kind` to a current `app_pid` via
  `BridgeService::find_clients_by_kind(kind)`. Pick one (e.g. first, or
  most-recently-active).
- Map descriptor `name` to a bridge `action` string. Probably just strip
  the namespace prefix: `browser.youtube.get_channel_metadata` →
  `GET_CHANNEL_METADATA`.
- Serialize `arguments` into the bridge payload string.
- Await `ResponseFrame`. Translate into chat `ToolResponse`.
- On `ToolCancel`, send `CancelFrame { id }` to the bridge client.

No protocol changes needed to `euro-bridge` itself — the existing
`Request`/`Response`/`Cancel` shape is the right primitive. New browser-side
handlers are added per tool action in `apps/browser`.

### `AcpClient` (new crate: `crates/app/euro-acp`)

Wraps the `agent-client-protocol` Rust SDK from Zed.

- Spawns the user-selected ACP agent (e.g. `opencode acp`).
- Performs `initialize` and caches declared capabilities.
- Holds a `(thread_id → session_id)` map; calls `session/new` on first use.
- Exposes a thin Rust API: `prompt`, `cancel`, `subscribe_updates`.
- v1 treats ACP as a **black box**: a single `acp.run` tool with
  `{ prompt, files? }` arguments, runs `session/prompt` to completion,
  returns the final text. ACP `session/update` events surface as
  `ToolProgress` frames.
- v2 (out of scope here): expose finer-grained ACP control as multiple
  tools, optionally let the server LLM interrupt the ACP agent mid-prompt
  by issuing `session/cancel` and re-prompting.

Permission flow: any `session/request_permission` from the ACP agent must
terminate at the user. `AcpClient` raises this as a `PermissionRequest`
back up the chat WS so the server LLM can see the request was made (and
choose to nudge the user), but the dialog is shown locally and the user
decides.

### `McpRegistry` (new crate: `crates/app/euro-mcp`, post-MVP)

Wraps an MCP client SDK (`rmcp` or equivalent).

- Reads configured-server list from user settings (command, args, env,
  transport).
- Spawns / connects to each server, supervises lifecycle (restart, backoff).
- Caches each server's `tools/list`; refreshes on
  `notifications/tools/list_changed`.
- Per-`(server_id, tool_name)` policy: `allow` / `ask_each_time` / `deny`.
- Tool descriptors fed into `Hello` are namespaced
  `mcp.{server_id}.{tool_name}` with `source: Mcp { server_id, tool_name }`.
- Defense-in-depth: even if a `ToolRequest` arrives for a tool not in the
  current `Hello` set, reject with `policy_denied`.

MCP-specific features deferred from v1: sampling (declined), resources
(later), prompts (later), roots (only when needed for FS-style servers).

## Permission model

The user is the final authority on what runs on their machine. This is
enforced at two layers:

1. **Schema layer.** The `Hello` frame contains only tools the user's
   policy currently allows. The LLM is never offered a forbidden tool, so
   it can't even try to call one.
2. **Defense-in-depth at receive time.** If a `ToolRequest` arrives for a
   tool not present in the last `Hello`, the client refuses with an error
   response (`code: policy_denied`). This catches schema drift, backend
   bugs, and stale state from prior turns.

For tools marked `requires_user_approval: true` in the descriptor, the
backend wraps each call in a `PermissionRequest` / `PermissionResponse`
round-trip before the call executes. Heuristic default: any tool whose name
matches `write|send|delete|create|update|exec|run` is marked
`requires_user_approval` unless explicitly overridden.

Policy storage lives in the existing Tauri settings infrastructure. The UI
lets the user view all available tools (first-party, bridge-attached,
ACP-attached, MCP-attached) and set the policy per tool or per source.

## End-to-end flows

### Server-local: Firecrawl scrape

```text
LLM emits tool_call { name: "scrape_url", args: { url } }
  agent_loop dispatches via ToolDispatcher
    source = ServerLocal
    BaseTool::invoke_tool_call(call).await  -- existing path
  result fed back into LLM context as ToolMessage
```

No WS traffic. No client involvement. Unchanged from today.

### Client-local: screenshot of focused window

```text
LLM emits tool_call { name: "screenshot", args: {} }
  agent_loop → ToolDispatcher
    source = ClientLocal
    RemoteToolBus::call(call_id, descriptor, args)
      → ChatServerMessage::ToolRequest sent on WS
      ← ChatClientMessage::ToolResponse { call_id, result: image bytes (base64) }
  result fed back to LLM
```

On the client:

```text
ChatBridge receives ToolRequest
  source = ClientLocal, name = "screenshot"
  ClientToolRegistry["screenshot"].invoke(args).await
    → native Tauri command, returns PNG bytes
  send ToolResponse
```

### Client-routed: YouTube channel metadata (the canonical example)

```text
LLM emits tool_call { name: "browser.youtube.get_channel_metadata", args: { channel_url } }
  agent_loop → ToolDispatcher
    source = Bridge { app_kind: "browser" }
    RemoteToolBus::call(...)
      → ChatServerMessage::ToolRequest
      ← ChatClientMessage::ToolResponse { call_id, result: { name, subscribers, ... } }
  result fed back to LLM
```

On the client:

```text
ChatBridge receives ToolRequest
  source = Bridge { app_kind: "browser" }
  BridgeAdapter::dispatch:
    app_pid = BridgeService::find_clients_by_kind("browser").first()
    action = "GET_CHANNEL_METADATA" (stripped from descriptor name)
    payload = serde_json::to_string(&args)
    BridgeService::send_request(app_pid, action, payload).await
      → RequestFrame down the loopback WS
        → euro-browser native-messaging host receives it
          → apps/browser extension's background script handles GET_CHANNEL_METADATA
            → scrapes YouTube DOM / hits YouTube Data API
          ← ResponseFrame with payload
        ← back through native-messaging host
      ← ResponseFrame on the loopback WS
    BridgeAdapter unpacks payload
  send ToolResponse on chat WS
```

The whole bridge chain is unchanged. Only the top — the chat WS leg — is new.

### ACP-piped: ask the local OpenCode to refactor a file

```text
LLM emits tool_call { name: "acp.run", args: { prompt: "refactor X to use Y", files: [...] } }
  agent_loop → ToolDispatcher
    source = Acp { session_id }
    RemoteToolBus::call(...)
      → ChatServerMessage::ToolRequest
      (during execution)
      ← ChatClientMessage::ToolProgress { call_id, payload: ACP session/update }  ...repeated
      ← ChatClientMessage::PermissionRequest (if ACP asks to write a file)
      → ChatServerMessage::PermissionResponse  (after user approves)
      ← ChatClientMessage::ToolResponse { call_id, result: { final_text, diffs } }
  result fed back to LLM
```

On the client:

```text
ChatBridge receives ToolRequest
  source = Acp { session_id }
  AcpClient::run(session_id, prompt, files).await
    streams session/update events → forwarded as ToolProgress
    handles fs/read_text_file, terminal/create, etc. locally per ACP spec
    on session/request_permission → bubble up as PermissionRequest, await response
  send ToolResponse with final result
```

## Capability handshake — concrete `Hello` shape

```json
{
  "type": "Hello",
  "client_capabilities": [
    {
      "name": "screenshot",
      "description": "Capture the user's currently focused window as a PNG.",
      "input_schema": { "type": "object", "properties": {} },
      "timeout_ms": 5000,
      "requires_user_approval": false,
      "source": "ClientLocal"
    },
    {
      "name": "browser.youtube.get_channel_metadata",
      "description": "Return the YouTube channel's subscriber count, ...",
      "input_schema": {
        "type": "object",
        "properties": { "channel_url": { "type": "string" } },
        "required": ["channel_url"]
      },
      "timeout_ms": 10000,
      "requires_user_approval": false,
      "source": { "Bridge": { "app_kind": "browser" } }
    },
    {
      "name": "acp.run",
      "description": "Delegate a sub-task to the user's attached coding agent.",
      "input_schema": { ... },
      "timeout_ms": 600000,
      "requires_user_approval": true,
      "source": { "Acp": { "session_id": "..." } }
    }
  ],
  "policy_summary": {
    "approval_required_count": 3,
    "blocked_count": 7
  }
}
```

`policy_summary` is informational — useful in logs and analytics. Authority
is the per-tool `requires_user_approval` flag.

## Implementation phases

### Phase 1: Vertical slice — client-routed YouTube example

Smallest possible end-to-end proof. Forces every protocol decision through
without ACP or MCP complications.

1. Extend `thread-core` with `ToolDescriptor`, `ToolSource`, the new
   `ChatServerMessage` / `ChatClientMessage` variants. Generate updated
   taurpc bindings (ask user to regenerate).
2. Implement `RemoteToolBus` and source-aware dispatch in `agent_loop.rs`.
3. Add `Hello` parsing in `handlers/chat.rs` ahead of the first
   `Send`/`Regenerate`.
4. Stand up `ChatBridge` in `euro-tauri` with `BridgeAdapter` only (no
   `ClientToolRegistry`, no ACP, no MCP yet).
5. Add a `GET_CHANNEL_METADATA` action in `apps/browser`.
6. Manually drive a chat turn that calls the tool; verify it runs end-to-end.

Exit criteria: a YouTube tab open, user asks "is this channel trustworthy,"
the backend LLM calls `browser.youtube.get_channel_metadata`, the response
flows back, the LLM produces an answer that cites the metadata.

### Phase 2: Client-local tools

Add `ClientToolRegistry` to `ChatBridge`. Wire up at least:

- `screenshot` (returns PNG bytes from focused window)
- `focus_state` (active app + window title)
- `active_app_metadata` (using existing focus tracker)

These let the LLM ground itself in user context without the user having to
describe what they're doing.

### Phase 3: Permission model UX

- Tool descriptor `requires_user_approval` enforcement on the client.
- `PermissionRequest` / `PermissionResponse` round-trip wired up.
- Native dialog in Tauri for approval requests.
- Settings UI for per-tool / per-source policy (allow / ask / deny).
- `Hello` filtering based on policy.

### Phase 4: ACP integration

- New crate `crates/app/euro-acp` wrapping `agent-client-protocol`.
- Subprocess lifecycle, session management.
- `AcpClient` registered as a source in `ChatBridge`.
- One tool exposed: `acp.run`, black-box semantics, progress streaming.
- Settings UI for selecting which ACP agent to use.

### Phase 5: MCP integration (open-ecosystem add-on)

- New crate `crates/app/euro-mcp` wrapping an MCP client SDK.
- Configured-server list, lifecycle supervision, policy storage.
- Tools surfaced into `Hello` as `mcp.{server}.{tool}` with `Mcp` source.
- Settings UI for adding/removing servers, importing Claude Desktop configs.

### Phase 6: Server-local tools beyond what's already there

- Firecrawl sidecar wired as a `ServerLocal` tool with the existing
  `BaseTool` shape.
- Any other server-resident tools the backend offers (cloud RAG over
  activity timeline, server-side image analysis, etc.).

Phase 6 has no protocol dependencies — it's parallel to all the others.

## Decisions to lock in early

These shape downstream work; settle them before Phase 1 lands.

1. **Tool name namespacing.** Recommended: dotted lowercase,
   `{source}.{subdomain}.{operation}`, e.g. `browser.youtube.get_channel_metadata`,
   `client.screenshot`, `mcp.github.create_issue`, `acp.run`. Stable across
   versions; namespace prefixes hint at source but never substitute for the
   descriptor's `source` field at dispatch time.
2. **One WS or multiplexed.** One. Extend the existing chat WS rather than
   opening a parallel "tool channel." Multiplex with `call_id`. Adding a
   parallel socket doubles failure modes and complicates reconnect.
3. **`Hello` is mandatory.** Server rejects the WS with a clear error if
   the first frame isn't `Hello`. The current contract (first frame is
   `Send`/`Regenerate`) becomes "first frame is `Hello`, second is
   `Send`/`Regenerate`."
4. **Backend never sees raw ACP / raw MCP frames.** Everything is
   normalized into `ToolRequest` / `ToolResponse` / `ToolProgress` /
   `PermissionRequest`. The client is the translation layer. This is what
   keeps the backend decoupled from ACP/MCP version drift.
5. **ACP is black-box in v1.** Single `acp.run` tool, no fine-grained
   exposure of `session/prompt` parameters or interrupts. Revisit in v2 if
   there's a use case.
6. **MCP sampling is declined.** If an MCP server sends `sampling/createMessage`,
   the client refuses politely. Letting an MCP server reach into the
   backend's LLM creates billing and trust problems that aren't worth
   v1.x complexity.
7. **Source of record for transcripts is the backend.** Client keeps no
   durable transcript state. Tool calls and results are persisted by the
   backend regardless of execution site.

## Open questions

- **Multi-bridge-client routing.** If the user has two browsers open (Chrome
    - Firefox) and both have the extension installed, both register as
      `app_kind: "browser"`. Which one does `BridgeAdapter` pick? Options:
      most-recently-active (needs activity tracking), explicit per-request
      selection, all-of-them-and-merge. Simplest v1: most recently active,
      fall back to first.
- **`Hello` re-issuance mid-session.** What happens when the user adds an
  MCP server or attaches an ACP agent partway through a chat? Options:
  send a fresh `Hello` between turns (catalog applies to next turn only),
  or hot-update mid-turn (more complex). Recommend per-turn applicability:
  client sends `Hello` ahead of every `Send`/`Regenerate`, not just at
  connect.
- **Tool result schema validation.** Should the client validate
  `ToolResponse` payloads against the descriptor's `output_schema` before
  sending? Defensive but adds latency. Recommend: yes, in dev builds; skip
  in release for performance, trust the executor.
- **Per-tool timeout granularity.** Single `timeout_ms` per descriptor, or
  separate timeouts for "first byte" vs. "total"? Long-running tools that
  stream progress benefit from the distinction. v1: single `timeout_ms`,
  reset on every `ToolProgress` frame ("inactivity timeout"). v2: explicit
  if needed.
- **Where does the LLM-visible tool schema diverge from the wire
  descriptor?** Some descriptor fields (`source`, `timeout_ms`,
  `requires_user_approval`) are routing concerns the LLM shouldn't see.
  The backend strips these when building the LLM-side tool list. Defined
  cleanly in `tool_catalog.rs`.

## What this rewrite is not

- Not a switch to MCP as the internal protocol. The chat-stream protocol
  stays Eurora-native; MCP is one _source_ of tools that the client
  translates locally.
- Not a redesign of `euro-bridge`. The bridge stays as-is; we layer a typed
  tool descriptor catalog above its existing `Request`/`Response`/`Cancel`
  primitive.
- Not a new transport. Same WebSocket at `/threads/{id}/chat`, same
  loopback bridge, same browser native-messaging chain.
- Not a permission-prompt-every-call system. The user pre-declares policy;
  per-call prompts are reserved for tools explicitly flagged as mutating
  the user's environment.

The principle this all rests on: **the LLM sees one tool catalog. Where each
tool runs is a routing decision the model never knows about.** Everything
else is plumbing that makes that true.
