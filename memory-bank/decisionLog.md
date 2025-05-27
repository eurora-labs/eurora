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

[2025-05-25 12:42:00] - Conducted comprehensive critical analysis of eur-activity crate

**Decision:** Performed detailed architectural analysis and identified critical issues in the eur-activity crate that prevent production use.

**Rationale:**

- The crate has fundamental compilation issues (invalid Rust edition "2024")
- Multiple runtime panic risks from todo!() implementations and expect() calls
- Poor error handling and hardcoded configuration values
- Missing comprehensive testing and documentation
- Memory and performance concerns with image handling

**Implementation Details:**

- Created critical-issues-analysis.md documenting 10 major issues with severity ratings
- Developed architecture-overview.md explaining the strategy pattern and component relationships
- Built implementation-roadmap.md with 4-phase development plan spanning 8-10 weeks
- Designed testing-strategy.md with comprehensive unit, integration, and performance testing approach
- Created README.md as documentation index with quick start guide and development guidelines
- All documentation includes proper file references and line numbers for traceability

[2025-05-25 12:45:45] - Updated eur-activity documentation based on user feedback

**Decision:** Corrected documentation to reflect that Rust edition 2024 is valid and extension IDs should remain hardcoded.

**Rationale:**

- Rust edition 2024 has been released and is valid for use
- Extension IDs are intentionally hardcoded as they serve as identifiers for specific asset processing pipelines
- The application architecture will support a large number of different asset types, each requiring specific processing identification
- Hardcoded IDs provide stable references for the asset processing system

**Implementation Details:**

- Removed Cargo.toml edition issue from critical issues analysis
- Updated priority recommendations to focus on TODO implementations and error handling
- Added explanation that extension IDs identify asset processing pipelines
- Clarified that extension IDs will remain hardcoded for architectural reasons
- Updated roadmap to include asset type registry and processing pipeline documentation
- Modified README to reflect corrected understanding of design decisions

[2025-05-25 12:55:55] - Completed Phase 1 implementation of eur-activity crate critical fixes

**Decision:** Successfully implemented all Phase 1 critical fixes for the eur-activity crate as planned.

**Rationale:**

- Eliminated all runtime panics by implementing missing TODO methods
- Added proper error handling with custom ActivityError types
- Implemented comprehensive timestamp tracking for snapshots
- Added basic unit test coverage (15 tests passing)
- Fixed parameter ordering bug in strategy selection

**Implementation Details:**

- Task 1: Added timestamp fields to ArticleSnapshot and YoutubeSnapshot structs with proper constructors
- Task 2: Implemented gather_state() method returning JSON representation of browser activity state
- Task 3: Created custom ActivityError enum with proper From implementations and safe image loading helper
- Task 4: Added 15 unit tests covering core functionality, strategy selection, and error handling
- Task 5: Skipped documentation comments per user preference to keep code clean
- Fixed critical bug in select_strategy_for_process parameter ordering
- All tests now pass without warnings

[2025-05-25 14:33:00] - Fixed Linux focus tracker to detect Chrome tab switches

**Decision:** Modified the Linux X11 focus tracker to monitor window title changes in addition to active window changes.

**Rationale:**

- The macOS implementation reports focus events for both window switches AND tab switches in Chrome
- The Linux implementation only reported focus events for window switches, missing Chrome tab switches
- This inconsistency meant the timeline tracking behaved differently across platforms
- Chrome tab switches change the window title but not the active window in X11

**Implementation Details:**

- Added tracking of the currently focused window (`current_focused_window`)
- Modified event handling to listen for both `_NET_ACTIVE_WINDOW` and `_NET_WM_NAME` property changes
- When active window changes: stop monitoring old window, start monitoring new window for title changes
- When title changes on focused window: emit focus event with updated title
- Maintains the same event-driven approach while capturing title changes within the same window
- Added proper cleanup of event monitoring when windows change focus

[2025-05-27 09:45:00] - Implemented JWT authentication for OCR service using tonic interceptor

**Decision:** Added JWT token validation to the OCR service to ensure only authenticated users can access image transcription functionality.

**Rationale:**

- Security requirement to protect OCR service from unauthorized access
- Consistent authentication mechanism across all backend services
- Leverages existing JWT infrastructure from the auth service
- Follows best practices for microservice authentication

**Implementation Details:**

- Added jsonwebtoken and serde dependencies to eur-ocr-service/Cargo.toml
- Created JWT Claims structure matching the auth service implementation
- Implemented JwtConfig for OCR service with shared secret configuration
- Added validate_token() function to decode and validate JWT tokens
- Created authenticate_request() function to extract Bearer tokens from request metadata
- Modified OcrService struct to include jwt_config field with constructor
- Updated transcribe_image() method to authenticate requests before processing
- Modified main.rs to share JWT configuration between auth and OCR services
- Added proper error handling with Status::unauthenticated for invalid tokens
- Included logging for authentication success/failure events

**Security Features:**

- Validates JWT signature using shared secret
- Ensures token type is "access" (not refresh)
- Checks token expiration automatically via jsonwebtoken library
- Extracts user information from validated claims for logging
- Returns proper gRPC status codes for authentication failures

[2025-05-27 09:51:00] - Refactored JWT authentication to use shared structures in eur-auth crate

**Decision:** Moved duplicated JWT structures and validation functions from individual services to a shared eur-auth crate to eliminate code duplication.

**Rationale:**

- Eliminates code duplication between auth-service and ocr-service
- Provides a single source of truth for JWT-related functionality
- Improves maintainability and consistency across services
- Follows DRY (Don't Repeat Yourself) principle
- Makes it easier to add JWT authentication to additional services

**Implementation Details:**

- Created shared JWT structures in crates/common/eur-auth/src/lib.rs:
    - Claims struct with all JWT fields
    - JwtConfig struct with secret and expiry configurations
    - validate_token() function for general token validation
    - validate_access_token() function specifically for access tokens
    - validate_refresh_token() function specifically for refresh tokens
- Updated eur-auth-service to use shared structures:
    - Added eur-auth dependency to Cargo.toml
    - Removed duplicated Claims and JwtConfig structs
    - Updated validate_token() method to use shared function
    - Cleaned up unused imports
- Updated eur-ocr-service to use shared structures:
    - Replaced local JWT dependencies with eur-auth dependency
    - Removed duplicated Claims, JwtConfig, and validate_token code
    - Updated authenticate_request() to use validate_access_token()
- Updated eur-monolith to use shared JwtConfig:
    - Added eur-auth dependency to Cargo.toml
    - Updated imports to use shared JwtConfig
    - Simplified service initialization with shared configuration

**Benefits:**

- Single point of maintenance for JWT functionality
- Consistent token validation across all services
- Easier to add new JWT-authenticated services
- Reduced codebase size and complexity
- Better type safety and consistency

[2025-05-27 11:24:00] - Transferred AuthService implementation to ProtoAuthService following OCR service pattern

**Decision:** Restructured the authentication service to move all business logic into the ProtoAuthService trait implementation, similar to how the OCR service is structured.

**Rationale:**

- Provides consistency across all gRPC services in the project
- Follows the established pattern from the OCR service implementation
- Centralizes all authentication logic within the gRPC trait implementation
- Improves maintainability by having a single, consistent service architecture

**Implementation Details:**

- Added missing RPC calls to proto/auth_service.proto:
    - `rpc Register (RegisterRequest) returns (LoginResponse);`
    - `rpc RefreshToken (RefreshTokenRequest) returns (LoginResponse);`
- Added corresponding message definitions:
    - `RegisterRequest` with username, email, password, and optional display_name
    - `RefreshTokenRequest` with refresh_token field
- Updated imports to include new proto message types
- Added Default implementation for AuthService with proper database connection
- Implemented missing trait methods in ProtoAuthService:
    - `register()` method that calls existing `register_user()` logic
    - `refresh_token()` method that calls existing `refresh_token()` logic
- Maintained all existing business logic and error handling
- Preserved JWT token generation and validation functionality
- Added proper gRPC status code handling for authentication errors

**Benefits:**

- Consistent service architecture across all backend services
- Complete gRPC API coverage for authentication operations
- Proper error handling with appropriate gRPC status codes
- Maintains existing security features and JWT functionality

[2025-05-27 11:27:00] - Fixed naming conflict and async issues in AuthService implementation

**Decision:** Resolved compilation errors in the AuthService by fixing method naming conflicts and removing problematic Default implementation.

**Rationale:**

- The refresh_token method name conflicted between the trait implementation and internal method
- The Default implementation tried to create a DatabaseManager synchronously, but DatabaseManager::new() is async
- Removing Default implementation ensures proper service initialization with database connections

**Implementation Details:**

- Renamed internal method from `refresh_token()` to `refresh_access_token()` to avoid naming conflict
- Updated trait implementation to call the renamed `refresh_access_token()` method
- Removed Default implementation that caused async/sync mismatch with DatabaseManager::new()
- Service now requires proper initialization through the `new()` constructor with database connection
- All compilation errors resolved, cargo check passes successfully

**Benefits:**

- Eliminates recursive method calls that would cause stack overflow
- Ensures proper async handling of database operations
- Maintains clean separation between trait methods and internal implementation
- Follows proper Rust patterns for service initialization
