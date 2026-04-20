# Reference code

These files are the pre-Phase-0 implementation of message handling and the `StateGraph` builder. They are **not** compiled into the crate (the `.rs.ref` extension hides them from Cargo) and exist only as a reference for the port in later phases.

- `message.rs.ref` — salvage target for **Phase 7** (`add_messages` reducer and `MessagesState`).
- `state.rs.ref` — reference only; **Phase 6** will rewrite `StateGraph` on top of Pregel, so treat the ergonomics here as a starting point rather than a blueprint.

Delete this directory once Phase 7 has landed.
