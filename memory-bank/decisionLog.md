# Decision Log

This file records architectural and implementation decisions using a list format.
2025-04-25 21:14:22 - Initial creation of Memory Bank.

## Database Implementation Decisions

[2025-04-28 17:17:30] - Implemented SQLite as the database engine for personal data

**Decision:** Used SQLite for storing personal data including activities, video frames, and text extracted from OCR.

**Rationale:**

- SQLite is well-suited for embedded applications and desktop software
- Lightweight with zero configuration and server-less operation
- Robust transaction support with ACID compliance
- Good performance for the expected data volume
- Cross-platform compatibility aligns with Tauri's multi-platform approach

**Implementation Details:**

- Created explicit relationships in the database schema (activity → activity_asset, etc.)
- Used UUID strings as primary keys for all tables to ensure uniqueness
- Added indexes on foreign keys to improve query performance
- Implemented a Rust interface with SQLX for type-safe database access
- Added comprehensive API for common database operations
- Used ISO8601 format for datetime fields to ensure compatibility
  2025-04-25 21:18:12 - Improvements to launcher components.
  2025-04-28 13:53:00 - Fixed async/await issue in OCR service.

## Decision

- Created Memory Bank to maintain project context and track development progress
- Structured Memory Bank with five core files: productContext.md, activeContext.md, progress.md, decisionLog.md, and systemPatterns.md
- Improved launcher components with several enhancements to fix bugs and improve user experience
- Fixed async/await issue in OCR service by properly handling futures

## Rationale

- A Memory Bank provides persistent context across different sessions and modes
- Structured documentation helps maintain clarity about project goals, progress, and decisions
- Enables more effective collaboration and knowledge transfer
- Fixing bugs and improving user experience in the launcher components enhances the overall application quality
- Implementing proper auto-scroll functionality ensures users can see new messages as they arrive
- Adding delete functionality for activity badges gives users more control over their interface
- Properly handling async/await in OCR service ensures correct processing of image recognition requests

## Implementation Details

- Created memory-bank directory at the root of the project
- Populated initial files with basic project information derived from repository structure
- Set up structure for ongoing updates as the project evolves
- Made the following improvements to launcher components:
  - Fixed auto-scroll functionality by adding a proper class and setTimeout approach
  - Implemented delete functionality for activity badges using array splicing
  - Cleaned up commented-out code to improve maintainability
  - Fixed duplicate function call in API key form
  - Used consistent event handling approaches across components
- Fixed OCR service by:
  - Adding futures crate as a dependency
  - Properly collecting futures into a vector
  - Using futures::future::join_all to await all futures concurrently

# Decision Log

This file tracks key architectural and design decisions made during the project's development.
2025-04-25 21:14:27 - Initial creation of Memory Bank.

[2025-05-14 18:53:02] - Implemented Context Chip component

**Decision:** Created a reusable Context Chip component based on the styling from Transcript.svelte.

**Rationale:**

- Extracted styling from Transcript.svelte to create a reusable component
- Implemented as a UI component following the same pattern as the Badge component
- Added variants to support different use cases (default, primary, secondary, destructive, outline)
- Included support for click handlers and links
- Used backdrop blur effect for a modern, translucent appearance

**Implementation Details:**

- Created component in packages/ui/src/custom-components/ui/context-chip
- Used tailwind-variants (tv) for variant styling
- Added proper TypeScript typing for variants
- Exported component in the main UI package index.ts
- Created documentation page in the docs app

[2025-05-17 14:19:55] - Modified Context Chip component for platform-specific styling

**Decision:** Updated the Context Chip component to use different background styles based on the platform.

**Rationale:**

- Linux desktop app requires a solid background due to platform-specific rendering differences
- Non-Linux platforms can benefit from the modern backdrop blur effect
- Maintaining consistent visual appearance across platforms while addressing platform-specific requirements

**Implementation Details:**

- Used CSS selectors to detect when the body has the "linux-app" class (Linux desktop app)
- For Linux: Applied solid background-color: rgba(0, 0, 0, 0.2) without backdrop filter
- For other platforms: Used transparent background with backdrop-filter: blur(6px)
- Ensured backward compatibility with existing usage of the component

[2025-05-24 12:41:02] - Migrated frontend from regular Tauri commands to TauRPC procedures

**Decision:** Updated the frontend TypeScript code to use TauRPC instead of direct Tauri invoke calls.

**Rationale:**
- TauRPC provides fully-typed IPC communication between Rust backend and TypeScript frontend
- Eliminates the need for manual type definitions and provides compile-time type safety
- Follows the existing pattern already established in the project with some procedures
- Improves developer experience with better autocomplete and error checking

**Implementation Details:**
- Updated `apps/desktop/src/routes/(launcher)/+page.svelte` to use TauRPC proxy
- Updated `apps/desktop/src/routes/(launcher)/api-key-form.svelte` to use TauRPC proxy
- Migrated the following function calls to TauRPC:
  - `check_api_key_exists()` → `taurpc.third_party.check_api_key_exists()`
  - `save_api_key()` → `taurpc.third_party.save_api_key()`
  - `initialize_openai_client()` → `taurpc.third_party.initialize_openai_client()`
  - `resize_launcher_window()` → `taurpc.window.resize_launcher_window()`
  - `send_query()` → `taurpc.send_query()` (for the main query functionality)
- Left some functions as fallbacks to regular invoke calls where TauRPC procedures don't exist yet:
  - `list_activities` (not yet implemented in TauRPC)
  - `list_conversations` (not yet implemented in TauRPC)
- Used existing TauRPC bindings generated in `packages/tauri-bindings/src/lib/gen/bindings.ts`
