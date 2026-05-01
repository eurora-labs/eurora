# Microsoft Word integration — phased plan

This plan adds a Word strategy to `euro-activity` so the AI can both observe Word
documents and control them. The mechanism is an Office.js add-in that lives
inside Word and speaks the existing bridge frame protocol back to the desktop
app, with the bridge generalized to serve clients other than browsers.

The phases below are ordered so each one is independently shippable and leaves
the tree green. Phase 1 is invasive but unblocks everything else; phases 2–4
are additive; phases 5–6 are productionization.

---

## Phase 1 — Generalize the bridge

**Goal.** The `BrowserBridgeService` is misnamed: its protobuf has no browser
semantics. Lift it into a generic `AppBridgeService`, add a WebSocket transport
that Office.js can speak, and implement the desktop-to-client request path that
is currently a stub (`euro-browser/src/server.rs:139` logs requests as
"unsupported"). No Word code lands in this phase.

**Deliverables.**

- New crate `crates/app/euro-bridge/` containing the renamed `AppBridgeService`.
- `proto/browser_bridge.proto` → `proto/app_bridge.proto`. Package renamed.
  `RegisterFrame` gains `client_kind: enum { Browser, Office, ... }` and an
  optional `auth_token: string`.
- WebSocket listener at `[::1]:1432` alongside the existing gRPC listener at
  `[::1]:1431`. Both feed the same frame router. Frames over WebSocket are
  length-prefixed protobuf.
- Outbound request path: `AppBridgeService::send_request(target_pid, action,
payload) -> ResponseFrame`. Backed by a `pending_outbound` correlation map
  symmetric to the existing inbound map. Includes timeouts and cancellation
  via `CancelFrame`.
- `crates/app/euro-browser/` becomes a thin shim depending on `euro-bridge`,
  registering only the browser-extension dispatchers. The native-messaging
  binary keeps working with at most a re-import.

**Acceptance.**

- Existing browser extension still functions end-to-end (tab events, metadata,
  asset retrieval) with no observable change.
- New unit tests in `euro-bridge` cover: WebSocket framing round-trip,
  outbound request/response correlation, outbound request timeout, cancel
  propagation in both directions.
- `cargo check --workspace` and `cargo test -p euro-bridge -p euro-browser`
  pass.

**Files.**

- New: `crates/app/euro-bridge/{Cargo.toml, src/lib.rs, src/server.rs,
src/websocket.rs, src/router.rs}`
- New: `proto/app_bridge.proto`
- Modified: `crates/app/euro-browser/src/{lib.rs, server.rs}` (slimmed)
- Modified: `crates/app/euro-tauri/Cargo.toml`, startup wiring
- Removed: `proto/browser_bridge.proto`

---

## Phase 2 — Process detection and strategy scaffolding

**Goal.** Teach the rest of the system that Word exists, without yet talking
to it. Lays the type-level groundwork so phase 3 is purely behavioral.

**Deliverables.**

- `crates/app/euro-process/src/office.rs` with an `OfficeProduct` enum
  (`Word`, `Excel`, `PowerPoint`, `Outlook`, `OneNote`) and
  `from_process_name`. Only `Word` is wired to a strategy; the rest exist so
  follow-on integrations don't require another refactor.
- `crates/app/euro-activity/src/strategies/word.rs` containing a `WordStrategy`
  that compiles, identifies Word via `OfficeProduct::Word`, and produces a
  minimal `Activity` from the focused-window title. No bridge traffic yet.
- `WordStrategy` added to the `ActivityStrategy` enum and inserted into the
  dispatch chain in `strategies.rs:86-95` between `BrowserStrategy` and
  `DefaultStrategy`.
- New `WordSnapshot` and `WordAsset` types added to the `ActivitySnapshot` and
  `ActivityAsset` enums in `euro-activity/src/types.rs`. Both implement the
  required traits with placeholder `construct_messages` (returns an empty
  `ContentBlocks` until phase 3 fills the data in).
- Asset-type-string registration in `euro-activity/src/storage.rs`.
- `NativeMessage::Word(WordPayload)` variant in
  `crates/common/euro-native-messaging/src/types.rs`, plus the matching arms
  in the `TryFrom<NativeMessage>` impls for both enums.

**Acceptance.**

- Focusing Word on Windows or macOS produces an `Activity` in the live feed
  with the document's window title (no path, no page, no selection yet).
- All exhaustive matches on `ActivitySnapshot` / `ActivityAsset` compile.
- `cargo check --workspace` passes; `cargo test -p euro-activity -p
euro-process` passes.

**Files.**

- New: `crates/app/euro-process/src/office.rs`
- New: `crates/app/euro-activity/src/strategies/word.rs`
- New: `crates/app/euro-activity/src/snapshots/word.rs`
- New: `crates/app/euro-activity/src/assets/word.rs`
- Modified: `crates/app/euro-activity/src/strategies.rs`,
  `src/types.rs`, `src/storage.rs`
- Modified: `crates/common/euro-native-messaging/src/types.rs`

---

## Phase 3 — Office.js add-in: observation MVP

**Goal.** Prove the transport end-to-end. Word reports its live state to the
desktop; the AI can see it as context but cannot yet act on it.

**Deliverables.**

- New top-level project `apps/word-addin/` (TypeScript + Webpack, the standard
  Office add-in scaffold).
    - `manifest.xml` — Task Pane add-in, `Hosts: Document`, permission
      `ReadWriteDocument`.
    - `src/main.ts` — opens `ws://127.0.0.1:1432`, sends `RegisterFrame` with
      `client_kind = Office`, `host_pid`, `app_pid`, then runs the
      request/event loop.
    - `src/events/{selection,document}.ts` — subscribe to
      `document.onSelectionChanged` and `document.onChanged`, emit
      `EventFrame { action: "SELECTION_CHANGED" | "DOCUMENT_CHANGED" |
"PAGE_CHANGED" }`.
    - `src/actions/get-document.ts`, `src/actions/get-selection.ts` — the two
      read-only actions needed in this phase.
- `WordStrategy` upgraded to subscribe to those events, fetch the snapshot via
  `bridge.send_request(pid, "GET_DOCUMENT", _)`, and emit
  `ActivityReport::NewActivity` and `WordSnapshot` updates the same way
  `BrowserStrategy` does for tabs.
- `WordSnapshot` populated with `document_title`, `document_path`,
  `current_page`, `page_count`, `cursor_offset`, `selection_text`,
  `visible_text`, `headings_outline`. `construct_messages` emits a JSON
  `ContentBlock` plus a plain-text block for selection/visible text, modelled
  on `ArticleSnapshot`.
- Sideload instructions for dev: a one-page `apps/word-addin/README.md` (only
  if the team needs it; skipped otherwise per the no-docs default).

**Acceptance.**

- With the add-in sideloaded, opening a `.docx` and moving the cursor produces
  live `WordSnapshot` updates visible in the desktop UI.
- The snapshot contains the correct document path on Windows and macOS.
- Closing the document or switching to a different Word window updates the
  active activity.
- Manual smoke test: snapshot data appears as context to the AI in a chat
  about the open document.

**Files.**

- New: `apps/word-addin/{manifest.xml, package.json, webpack.config.js,
tsconfig.json, src/**}`
- Modified: `crates/app/euro-activity/src/strategies/word.rs`
- Modified: `crates/app/euro-activity/src/snapshots/word.rs`,
  `src/assets/word.rs` — populate fields, real `construct_messages`

---

## Phase 4 — AI control surface

**Goal.** The AI can drive Word, not just watch it. This is the first place in
the codebase where a `BaseTool` actually mutates external application state.

**Deliverables.**

- New module `crates/common/agent-chain-core/src/tools/word/` with one
  `StructuredTool` per action below. Each tool resolves the active Word PID
  from `WordStrategy::active_pid()`, calls
  `AppBridgeService::send_request(pid, action, payload)`, and returns the
  response payload as `ToolOutput::Json`.

    **Read tools:**
    - `word_get_document` → `GET_DOCUMENT`
    - `word_get_selection` → `GET_SELECTION`
    - `word_get_page` → `GET_PAGE`
    - `word_search` → `SEARCH`

    **Mutation tools:**
    - `word_navigate` → `NAVIGATE`
    - `word_insert_text` → `INSERT_TEXT`
    - `word_replace_range` → `REPLACE_RANGE`
    - `word_format_range` → `FORMAT_RANGE`
    - `word_add_comment` → `ADD_COMMENT`
    - `word_track_changes_toggle` → `TOGGLE_TRACK_CHANGES`
    - `word_save` → `SAVE`

- Mirror handlers in `apps/word-addin/src/actions/`, each a thin wrapper over
  a `Word.run(async ctx => ...)` block.
- Tool registration: tools are exposed only when `WordStrategy` is the active
  strategy, so they never appear in chats about an unrelated app. The agent
  layer already iterates `BaseTool` registries; we add a context-aware filter.
- Integration tests in `euro-bridge` that mock the add-in side and exercise
  each action with realistic payloads.

**Acceptance.**

- The AI, given a chat message like "make the second paragraph bold," issues
  `word_search` then `word_format_range` and the document updates.
- Each tool round-trips under 200ms on a warm bridge.
- Tool errors (bad range, invalid args, add-in disconnected) surface as
  structured `ToolOutput` errors, not panics.

**Files.**

- New: `crates/common/agent-chain-core/src/tools/word/{mod.rs,
document.rs, selection.rs, page.rs, search.rs, navigate.rs, insert.rs,
replace.rs, format.rs, comment.rs, track_changes.rs, save.rs}`
- New: `apps/word-addin/src/actions/{get-page.ts, search.ts, navigate.ts,
insert.ts, replace.ts, format.ts, comment.ts, track-changes.ts, save.ts}`
- Modified: agent tool registry / dispatcher (exact location depends on phase
  1 review of `agent-chain-core`)

---

## Phase 5 — Packaging and distribution

**Goal.** A user who installs Eurora gets the Word integration without any
manual sideload step.

**Deliverables.**

- **Windows MSI.** Add a `WordAddInRegistry` component to `fragment.wxs`:
    - Drop the manifest XML into `%APPDATA%\Microsoft\AddIns\Wef\`.
    - Register a per-user trust catalog entry pointing at it.
    - Mirror the existing pattern used for Chrome/Edge/Firefox native-messaging
      host registration.
- **macOS DMG.** Post-install script copies the manifest into
  `~/Library/Containers/com.microsoft.Word/Data/Documents/wef/`.
- **Linux.** No-op. `WordStrategy` compiles but `OfficeProduct::Word` never
  matches a process there.
- The add-in's static assets (HTML/JS bundle) are packaged inside the Tauri
  app and served from a small embedded loopback HTTP origin, so the manifest
  can reference a stable `https://localhost:<port>/...` URL the desktop owns.
- The release script `scripts/release.sh` and CI workflow
  `.github/workflows/publish.yaml` are extended to build the add-in bundle
  and include it in each platform's installer.

**Acceptance.**

- Installing the MSI on a clean Windows VM and launching Word automatically
  shows the Eurora task pane add-in.
- Same on macOS via the DMG.
- Uninstalling removes the manifest entries.

**Files.**

- Modified: `crates/app/euro-tauri/fragment.wxs`,
  `crates/app/euro-tauri/tauri.conf*.json`
- Modified: `scripts/release.sh`, `.github/workflows/publish.yaml`
- New: `apps/word-addin/build/` output included in the bundle

---

## Phase 6 — Hardening

**Goal.** Production-grade behavior under adversarial and degraded conditions.

**Deliverables.**

- **Auth.** Random per-install token stored in `euro-secret`. Required in
  `RegisterFrame.auth_token`. Bridge rejects connections without it. Closes
  the gap that any localhost process can drive Word through us today.
- **Reconnect.** Add-in transparently reconnects if the desktop restarts.
  Desktop side reconciles in-flight requests with `CancelFrame` on
  disconnect.
- **Backpressure.** Bound the per-client outbound queue and drop the oldest
  events under sustained load (selection-changed can fire fast during typing).
- **Timeouts.** Per-action timeouts in the tools (default 5s, override per
  tool). On timeout, send `CancelFrame` so the add-in can abort its
  `Word.run` block.
- **Telemetry.** Bridge emits structured logs for each request: action,
  client_kind, latency, outcome. Wired to the existing logging stack.
- **Integration tests.** End-to-end test that spawns a fake Office.js client
  (a small Rust binary that speaks the WebSocket protocol), registers, and
  exercises the full tool surface.

**Acceptance.**

- Killing and restarting the desktop reconnects the add-in within 2s with no
  user intervention.
- A localhost process attempting to register without the token is rejected.
- Soak test: 30 minutes of typing in Word produces no unbounded memory growth
  or dropped activity reports.

**Files.**

- Modified: `crates/app/euro-bridge/src/{server.rs, router.rs}`
- Modified: `crates/app/euro-bridge/Cargo.toml` (testing deps)
- New: `crates/app/euro-bridge/tests/word_e2e.rs`

---

## Notes on sequencing

Phase 1 is the only phase that touches existing browser code; everything after
it is additive. Phases 2 and 3 can land in the same release if the add-in is
ready, but they're separated here because Phase 2 is a pure-Rust change that
can be reviewed independently. Phase 4 depends on Phase 3 (tools need a live
add-in to test against). Phases 5 and 6 are independent of each other and can
be parallelized once Phase 4 lands.
