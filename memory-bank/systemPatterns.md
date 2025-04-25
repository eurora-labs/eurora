# System Patterns

This file documents recurring patterns and standards used in the project.
It is optional, but recommended to be updated as the project evolves.
2025-04-25 21:14:30 - Initial creation of Memory Bank.

## Coding Patterns

* Monorepo structure using pnpm workspaces for JavaScript/TypeScript packages
* Rust crates organized by functionality
* Protocol definitions in proto/ directory
* Tauri for desktop application framework (Rust backend + web frontend)

## Architectural Patterns

* Separation of concerns:
  * Frontend components in packages/
  * Backend services in crates/
  * Protocol definitions in proto/
* Modular architecture with specific components for:
  * Screen capture
  * Conversation management
  * Timeline/focus tracking
  * AI integration

## Testing Patterns

* To be determined as the project evolves