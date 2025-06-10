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

[2025-05-27 12:32:35] - Implemented GitHub Actions deployment workflow for eur-monolith backend service

**Decision:** Created a comprehensive CI/CD pipeline for the eur-monolith Rust backend service following the pattern of the existing deploy-web.yml workflow.

**Rationale:**

- The eur-monolith is a gRPC server combining OCR and Auth services that requires different deployment approach than the static web app
- Needed automated building, testing, and containerization for the Rust backend service
- Following established patterns from the project's existing Docker workflow (push-e2e-img.yml)
- Enables consistent deployment process with proper artifact management and Docker image publishing

**Implementation Details:**

- Created `.github/workflows/deploy-monolith.yml` with three main jobs: build, docker, and deploy
- Build job: Sets up Rust toolchain, runs formatting/linting checks, executes tests, and builds release binary
- Docker job: Creates optimized Debian-based container image with security best practices (non-root user, health checks)
- Deploy job: Provides deployment instructions and placeholder for actual deployment steps
- Triggers on changes to monolith crate and related dependencies (auth-service, ocr-service, remote-db, proto)
- Uses GitHub Container Registry (ghcr.io) for Docker image storage following project conventions
- Includes proper caching for Rust dependencies and Docker layers for faster builds
- Provides comprehensive environment variable documentation for deployment configuration
- Follows security best practices with minimal container image and non-root execution

**Key Features:**

- Automated Rust code quality checks (formatting, clippy, tests)
- Multi-stage workflow with artifact passing between jobs
- Docker image optimization with health checks and security hardening
- Flexible deployment target support (placeholder for various deployment methods)
- Proper environment variable handling for database connections and JWT configuration
- Concurrent deployment protection and manual workflow dispatch capability

[2025-05-28 09:56:18] - Implemented comprehensive registration page for Eurora web application

**Decision:** Created a modern, accessible registration form with comprehensive validation and user experience features.

**Rationale:**

- The existing registration page was just a placeholder with minimal functionality
- Users need a proper registration flow to create accounts for the Eurora platform
- Form should match the project's design system and use existing UI components
- Registration should align with the auth service proto definition (username, email, password, display_name)
- Modern UX patterns improve user conversion and reduce registration friction

**Implementation Details:**

- Built using existing UI components from @eurora/ui package (Card, Input, Label, Button, Alert)
- Implemented client-side validation for all required fields:
    - Username: minimum 3 characters, required
    - Email: valid email format validation, required
    - Password: minimum 8 characters, required
    - Confirm Password: must match password
    - Display Name: optional field
- Added password visibility toggles for better UX
- Included loading states with spinner during form submission
- Error handling with user-friendly error messages
- Success state with confirmation message and redirect to login
- Responsive design that works on mobile and desktop
- Proper accessibility with labels, ARIA attributes, and keyboard navigation
- SEO optimization with proper meta tags and title
- Form data structure matches RegisterRequest proto definition
- Added placeholder for actual API integration with auth service

**Benefits:**

- Professional user registration experience
- Reduces user errors with real-time validation
- Consistent with project's design system
- Accessible to users with disabilities
- Mobile-friendly responsive design
- Ready for integration with backend auth service

[2025-05-28 10:04:59] - Updated registration page to use improved form validation pattern with Svelte 5 syntax

**Decision:** Enhanced the registration form with better validation patterns and proper Svelte 5 event handling syntax.

**Rationale:**

- User requested to use shadcn-svelte form patterns instead of basic form implementation
- Project uses Svelte 5 which requires updated event handling syntax (onsubmit instead of on:submit)
- Better user experience with real-time field validation and visual feedback
- Follows existing project patterns seen in api-key-form.svelte

**Implementation Details:**

- Updated event handlers to use Svelte 5 syntax: onsubmit, onblur instead of on:submit, on:blur
- Implemented per-field validation with real-time feedback on blur events
- Enhanced password validation with stronger requirements (uppercase, lowercase, number)
- Added username validation with character restrictions (alphanumeric, hyphens, underscores)
- Visual error states with red borders and error messages
- Improved accessibility with aria-labels for password visibility toggles
- Better UX with disabled submit button when validation errors exist
- Maintained existing UI component usage pattern from the project

**Benefits:**

- Proper Svelte 5 compatibility without syntax errors
- Better user experience with immediate validation feedback
- Stronger security with enhanced password requirements
- Consistent with project's existing form patterns
- Accessible design with proper ARIA attributes

[2025-05-28 10:28:23] - Implemented gRPC-Web auth service integration for frontend registration

**Decision:** Created a complete gRPC-Web client service to connect the frontend registration form to the backend auth service.

**Rationale:**

- User requested integration with the existing backend auth service using grpc-web
- Need to provide real authentication functionality instead of simulated API calls
- Frontend should communicate directly with the gRPC backend service
- Token management needed for maintaining user sessions

**Implementation Details:**

- Created `apps/web/src/lib/services/auth-service.ts` with gRPC-Web client implementation
- Used official grpc-web package instead of @improbable-eng/grpc-web
- Generated TypeScript protobuf files using existing proto compilation scripts
- Updated `packages/proto/src/index.ts` to export auth service types and client
- Implemented custom GrpcWebRpc transport using fetch API with proper headers
- Created AuthService class with register(), login(), and refreshToken() methods
- Added TokenStorage utility class for localStorage token management
- Integrated auth service into registration page replacing simulated API call
- Added proper error handling and user-friendly error messages
- Included logging for debugging and monitoring

**Technical Features:**

- Type-safe gRPC communication using generated TypeScript types
- Automatic token storage and management in localStorage
- Error extraction and user-friendly error messages
- Support for username/email login and registration
- JWT token refresh functionality
- Proper gRPC-Web headers and content types

**Benefits:**

- Real backend integration with type safety
- Secure token-based authentication
- Consistent error handling across the application
- Ready for production use with proper token management
- Extensible for additional auth features (login, logout, etc.)

[2025-05-29 11:08:51] - Implemented comprehensive login page for Eurora web application

**Decision:** Created a modern, accessible login form following the same patterns and design as the registration page.

**Rationale:**

- Users need a proper login flow to access their existing Eurora accounts
- Login page should provide consistent user experience with the registration page
- Form should integrate with the existing gRPC-Web auth service for real authentication
- Should follow established project patterns for validation, error handling, and UI components
- Simplified form compared to registration (only login/email and password fields needed)

**Implementation Details:**

- Built using existing UI components from @eurora/ui package (Card, Input, Label, Button)
- Implemented client-side validation for required fields (login and password)
- Added password visibility toggle for better user experience
- Included loading states with spinner during form submission
- Error handling with user-friendly error messages from auth service
- Success state with confirmation message and automatic redirect to /app
- Responsive design that works on mobile and desktop
- Proper accessibility with labels, ARIA attributes, and keyboard navigation
- SEO optimization with proper meta tags and title
- Integration with existing auth service login() method and TokenStorage
- Uses Svelte 5 syntax (onsubmit, onblur) for event handling
- Automatic token storage in localStorage upon successful login
- Link to registration page for new users

**Benefits:**

- Complete authentication flow for existing users
- Consistent design and user experience with registration
- Real backend integration with type-safe gRPC communication
- Accessible design following web standards
- Mobile-friendly responsive layout
- Secure token-based authentication with automatic storage
- Ready for production use with proper error handling

[2025-06-01 15:40:00] - Created comprehensive authentication and token management integration plan for eur-tauri

**Decision:** Developed detailed architectural plan for integrating JWT-based authentication into the Eurora Tauri desktop application.

**Rationale:**

- Leverage existing eur-auth-service backend and eur-auth shared utilities
- Provide secure token storage using OS-level credential management via eur-secret
- Implement automatic token refresh to maintain seamless user experience
- Follow established TauRPC patterns for type-safe frontend-backend communication
- Ensure security best practices with proper token validation and session management

**Implementation Details:**

- 6-phase implementation plan spanning 4 weeks
- Core infrastructure: Auth procedures module, token manager, gRPC client integration
- Secure storage: Extend eur-secret for JWT token storage with OS keychain integration
- State management: Global auth state with reactive updates across the application
- Frontend integration: Auth context provider and service layer for UI components
- Background services: Automatic token refresh service with configurable intervals
- Protected APIs: Authenticated HTTP client for secure API communication
- Security considerations: Platform-specific secure storage, TLS enforcement, session management
- Testing strategy: Unit, integration, and performance tests for all auth components
- Migration strategy: Graceful transition for existing users with backward compatibility

[2025-06-01 15:45:00] - Revised authentication plan to use Rust-centric, stateless frontend approach

**Decision:** Completely revised the authentication integration plan to follow a Rust-centric architecture where eur-auth manages all token operations autonomously and the frontend remains stateless.

**Rationale:**

- Frontend should not store any authentication state or tokens
- eur-auth crate should be the single source of truth for all authentication operations
- Other services should request authentication from eur-auth just before making API calls
- eur-auth should automatically handle token refresh and provide valid tokens transparently
- Simpler, more secure architecture with clear separation of concerns

**Implementation Details:**

- Enhanced eur-auth crate with AuthManager that handles all token operations
- AuthProvider service that other procedures use to get valid authentication headers
- Stateless frontend that only queries auth state from Rust backend
- Automatic token refresh handled transparently by eur-auth before token expiration
- All API procedures request authentication just before making external calls
- Frontend uses reactive queries to get auth state from backend rather than storing it
- Centralized token storage using eur-secret with no frontend token exposure
- Clean separation: frontend triggers auth actions, Rust manages all auth state and tokens

[2025-06-02 14:52:40] - Implemented comprehensive Storybook setup for UI components

**Decision:** Created comprehensive Storybook stories for Button and VideoCard components in the packages/ui library.

**Rationale:**

- Storybook provides an isolated development environment for UI components
- Enables visual testing and documentation of component variants and states
- Improves component development workflow and collaboration
- Provides interactive playground for testing component behavior
- Supports design system documentation and component library maintenance

**Implementation Details:**

- Created [`packages/ui/src/stories/button/Button.stories.svelte`](packages/ui/src/stories/button/Button.stories.svelte:1) with comprehensive button component stories:
    - All variants (default, destructive, outline, secondary, ghost, link)
    - All sizes (sm, default, lg, icon)
    - Icon combinations and loading states
    - Disabled states and link behavior
    - Interactive controls for testing
- Created [`packages/ui/src/stories/video-card/VideoCard.stories.svelte`](packages/ui/src/stories/video-card/VideoCard.stories.svelte:1) with video card component stories:
    - Left and right alignment options
    - Multiple video format support (MP4, WebM)
    - Responsive layout demonstrations
    - Custom styling examples
    - Component composition breakdown
    - Fallback behavior for missing video sources
- Used existing Storybook configuration in [`packages/ui/.storybook/main.ts`](packages/ui/.storybook/main.ts:5) which already included proper story path patterns
- Leveraged `@storybook/addon-svelte-csf` for Svelte 5 compatibility
- Included proper TypeScript typing and component documentation
- Used sample video URLs from Google's test video bucket for demonstrations

**Benefits:**

- Visual component library for design system consistency
- Interactive testing environment for component development
- Documentation for component usage and API
- Quality assurance through visual regression testing capabilities
- Improved developer experience with component playground

[2025-06-02 15:49:40] - Created comprehensive Storybook documentation and guidelines

**Decision:** Analyzed existing Storybook implementation and created detailed guidelines and issue documentation for UI component stories.

**Rationale:**

- Existing Storybook stories showed inconsistent patterns and incomplete implementations
- Need standardized guidelines to ensure consistent, high-quality component documentation
- Critical issues identified that prevent effective component usage and development
- Documentation serves both current development and future team members

**Implementation Details:**

- Created `packages/ui/src/stories/RULES.md` with comprehensive guidelines covering:
    - File structure and organization standards
    - Story file structure requirements
    - Category hierarchy and naming conventions
    - Required stories for all components
    - Documentation standards and accessibility requirements
    - Technical requirements and performance considerations
- Created `packages/ui/src/stories/ISSUES.md` documenting 10 identified issues:
    - Critical: Incomplete ContextChip story, inconsistent categorization, import path issues
    - Moderate: Redundant launcher files, missing argTypes, inconsistent layouts
    - Minor: Container width inconsistencies, missing error states, accessibility gaps
- Analysis based on existing stories: Button, Command, ContextChip, and VideoCard
- Guidelines follow Storybook best practices and Svelte 5 patterns
- Prioritized recommendations for immediate, medium-term, and future improvements

[2025-06-02 18:38:00] - Enhanced launcher Storybook stories with background image integration

**Decision:** Added sample_background.jpg as a background image to all launcher component stories in Storybook.

**Rationale:**

- Improves visual presentation of launcher components in Storybook documentation
- Provides realistic context showing how components appear against actual backgrounds
- Enhances the developer experience when reviewing component designs
- Follows established Storybook guidelines for meaningful content and visual consistency
- Uses backdrop blur and semi-transparent overlays for modern, professional appearance

**Implementation Details:**

- Applied background image to all launcher story files: Command.stories.svelte, CommandGroup.stories.svelte, CommandInput.stories.svelte, CommandItem.stories.svelte
- Used CSS background-image with url('sample_background.jpg') referencing the static asset
- Implemented layered approach with absolute positioning for background and relative positioning for content
- Added semi-transparent white background (bg-white/20) with backdrop blur (backdrop-blur-[36px]) to command components
- Maintained appropriate minimum heights for different story types (200px-400px based on content)
- Preserved all existing functionality and component behavior
- Followed RULES.md guidelines for consistent container widths and spacing
- Used responsive design patterns with proper overflow handling

**Benefits:**

- Enhanced visual appeal of Storybook documentation
- Better representation of real-world usage scenarios
- Improved component contrast and readability testing
- Professional presentation for design reviews and stakeholder demonstrations
- Consistent visual treatment across all launcher component stories

[2025-06-03 08:17:50] - Created reusable StoryContainer component for Storybook stories

**Decision:** Extracted repeated container markup from launcher story files into a reusable [`StoryContainer.svelte`](packages/ui/src/stories/StoryContainer.svelte) component.

**Rationale:**

- Eliminates code duplication across multiple story files
- Provides consistent background styling and layout for all stories
- Improves maintainability by centralizing container logic
- Adds accessibility improvements with proper ARIA attributes
- Includes fallback background options for better reliability

**Implementation Details:**

- Created [`StoryContainer.svelte`](packages/ui/src/stories/StoryContainer.svelte:1) with slot-based children rendering
- Added CSS fallback background color (#f5f5f5) and secondary fallback image (grid pattern)
- Marked background div with `role="presentation"` and `aria-hidden="true"` for accessibility
- Updated all launcher story files to use the new component:
    - [`CommandItem.stories.svelte`](packages/ui/src/stories/launcher/CommandItem.stories.svelte:1)
    - [`CommandGroup.stories.svelte`](packages/ui/src/stories/launcher/CommandGroup.stories.svelte:1)
    - [`CommandInput.stories.svelte`](packages/ui/src/stories/launcher/CommandInput.stories.svelte:1)
    - [`Command.stories.svelte`](packages/ui/src/stories/launcher/Command.stories.svelte:1)
- Replaced 18+ instances of duplicated container markup with single component usage
- Maintained existing styling and functionality while reducing code duplication

**Benefits:**

- Reduced code duplication by ~500 lines across story files
- Centralized background styling and accessibility features
- Easier maintenance and updates to story container styling
- Consistent user experience across all Storybook stories
- Better accessibility compliance with proper ARIA attributes

[2025-06-03 13:21:00] - Implemented modern onboarding UI with card-based layout

**Decision:** Created a visually pleasing onboarding page with centered title and side-by-side clickable cards for authentication and local connection options.

**Rationale:**

- User requested specific layout: title in middle top, left card for "Log in or Sign up", right card for "Connect to local"
- Needed to follow existing project design patterns using backdrop blur effects and Tailwind CSS
- Cards should be clickable and visually appealing to improve user experience
- Design should be consistent with the project's existing launcher component styling

**Implementation Details:**

- Used full-screen layout with backdrop blur effect matching the launcher component pattern
- Implemented responsive grid layout (1 column on mobile, 2 columns on desktop)
- Added gradient background with blue-to-purple theme for visual appeal
- Created two distinct cards with different color schemes:
    - Left card: Blue theme for authentication (login/signup)
    - Right card: Green theme for local connection
- Added hover effects and transitions for better interactivity
- Used existing UI components from @eurora/ui package (Card, Button)
- Maintained Linux compatibility with fallback styling (no backdrop filter)
- Added proper spacing and typography hierarchy with centered title
- Implemented proper semantic structure with Card.Header, Card.Content sections

**Benefits:**

- Professional and modern user interface
- Clear visual distinction between the two main onboarding paths
- Responsive design that works on different screen sizes
- Consistent with project's existing design system
- Accessible and user-friendly layout
- Smooth animations and hover effects for better UX

[2025-06-05 15:57:40] - Implemented comprehensive Google OAuth integration for eur-auth-service

**Decision:** Split Google OAuth implementation into two parts: URL generation and callback handling with third-party login integration.

**Rationale:**

- Separation of concerns: URL generation for redirecting users vs. callback handling for processing authorization codes
- Integration with existing auth service architecture using third-party credentials
- Follows OAuth 2.0 authorization code flow best practices
- Enables seamless user registration and login via Google accounts

**Implementation Details:**

- **URL Generation (`google.rs`):**

    - Created `GoogleOAuthClient` for generating authorization URLs
    - Uses oauth2 crate with proper Google OAuth endpoints
    - Requests OpenID Connect scopes (openid, email, profile)
    - Returns authorization URL and CSRF state token

- **Callback Handler (`callback_handler.rs`):**

    - Created Axum-based HTTP server to handle OAuth callbacks
    - Exchanges authorization code for access tokens
    - Fetches user info from Google's userinfo endpoint
    - Calls auth service login with third-party credentials

- **Auth Service Integration:**

    - Implemented `handle_third_party_login()` method in AuthService
    - Validates Google ID tokens by fetching user info
    - Creates new users automatically if they don't exist
    - Generates JWT tokens for authenticated users
    - Supports both existing and new user flows

- **Dependencies Added:**
    - `reqwest` for HTTP requests to Google APIs
    - `serde_urlencoded` for parsing callback query parameters
    - `url` for URL parsing utilities

**Benefits:**

- Complete OAuth 2.0 flow implementation
- Automatic user registration for new Google users
- Secure token validation and user authentication
- Extensible architecture for additional OAuth providers (GitHub, etc.)
- Integration with existing JWT token system

[2025-06-06 11:37:00] - Implemented OAuth state management for secure third-party authentication

- Added oauth_state table support in eur-remote-db with PKCE verifier storage
- Modified get_third_party_auth_url to generate and store temporary state/verifier pairs
- Enhanced handle_google_login to validate state parameters against stored values
- Updated Google OAuth client to support custom state parameters
- Implemented state consumption to prevent replay attacks
- Added proper CSRF protection for OAuth callback flow

[2025-06-10 14:31:00] - Refactored PDF and Article watchers to follow YouTube watcher architectural pattern

**Decision:** Migrated PDF and Article content script watchers to extend the base Watcher class from chrome-ext-shared package, following the same pattern as the YouTube watcher.

**Rationale:**

- Establishes consistent architecture across all content script watchers
- Leverages shared base class for common functionality (listen, handleNew, handleGenerateAssets, handleGenerateSnapshot)
- Improves maintainability by centralizing watcher behavior patterns
- Follows DRY principle by eliminating duplicated message handling logic
- Provides type safety through shared interfaces and message types

**Implementation Details:**

- **PDF Watcher Refactoring:**

    - Created `types.ts` with PdfMessageType and WatcherParams interfaces
    - Refactored PdfWatcher class to extend Watcher<WatcherParams>
    - Implemented all required abstract methods from base class
    - Added chrome-ext-shared dependency to package.json
    - Updated vite.config.ts with chrome-ext-shared alias
    - Maintained existing PDF parsing functionality using PDFViewerApplication

- **Article Watcher Refactoring:**
    - Created `types.ts` with ArticleMessageType and WatcherParams interfaces
    - Refactored ArticleWatcher class to extend Watcher<WatcherParams>
    - Implemented all required abstract methods from base class
    - Added chrome-ext-shared dependency to package.json
    - Updated vite.config.ts with chrome-ext-shared alias
    - Used create() function with ProtoNativeArticleAssetSchema for proper protobuf message creation
    - Maintained existing Readability-based article parsing functionality

**Benefits:**

- Consistent message handling across all content script watchers
- Shared type definitions prevent interface mismatches
- Easier to add new watcher types following established pattern
- Centralized error handling and response patterns
- Better code organization and separation of concerns
