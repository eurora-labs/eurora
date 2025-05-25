# Issues List: eur-native-messaging Crate

## High Priority Issues

### ISSUE-001: Unsafe unwrap() Usage in JSON Processing
- **File**: [`asset_context.rs`](../src/asset_context.rs)
- **Lines**: 36-49, 84-104, 146-159
- **Severity**: HIGH
- **Type**: Error Handling
- **Description**: Extensive use of `unwrap()` when accessing JSON fields without validation
- **Impact**: Application crashes when browser extension sends malformed or missing data
- **Fix Required**: Replace with proper error handling using `Result` types and validation

### ISSUE-002: Unsafe unwrap() Usage in Snapshot Processing
- **File**: [`snapshot_context.rs`](../src/snapshot_context.rs)
- **Lines**: 16-26, 33-35
- **Severity**: HIGH
- **Type**: Error Handling
- **Description**: Base64 decoding and JSON field access using `unwrap()`
- **Impact**: Runtime panics on invalid data
- **Fix Required**: Implement proper error propagation and validation

### ISSUE-003: Potential Deadlock in stdio Handling
- **File**: [`server.rs`](../src/server.rs)
- **Lines**: 93-94
- **Severity**: HIGH
- **Type**: Concurrency
- **Description**: Acquiring multiple mutexes simultaneously without proper ordering
- **Impact**: Potential deadlock under concurrent load
- **Fix Required**: Implement proper mutex ordering or use single mutex

### ISSUE-004: Missing Unit Tests
- **File**: Entire crate
- **Severity**: HIGH
- **Type**: Testing
- **Description**: No test coverage for critical functionality
- **Impact**: Undetected bugs and regressions
- **Fix Required**: Implement comprehensive unit test suite

### ISSUE-005: No Input Validation
- **File**: All converter modules
- **Severity**: HIGH
- **Type**: Security/Validation
- **Description**: No validation of incoming JSON data structure or content
- **Impact**: Runtime errors, potential security issues
- **Fix Required**: Add comprehensive input validation layer

## Medium Priority Issues

### ISSUE-006: Protocol Field Name Inconsistency
- **File**: [`tauri_ipc.proto`](../../../proto/tauri_ipc.proto) line 68 vs [`asset_context.rs`](../src/asset_context.rs) line 152
- **Severity**: MEDIUM
- **Type**: Protocol
- **Description**: Field naming mismatch (`selectedText` vs `selected_text`)
- **Impact**: Potential data conversion issues
- **Fix Required**: Standardize field naming across protocol definitions

### ISSUE-007: Channel Buffer Size Limitations
- **File**: [`server.rs`](../src/server.rs)
- **Lines**: 60-61, 195
- **Severity**: MEDIUM
- **Type**: Performance
- **Description**: Fixed buffer sizes may cause blocking or message loss under high load
- **Impact**: Performance degradation or message loss
- **Fix Required**: Implement dynamic buffer sizing or backpressure handling

### ISSUE-008: Lock File Race Condition
- **File**: [`main.rs`](../src/main.rs)
- **Lines**: 139-144
- **Severity**: MEDIUM
- **Type**: Resource Management
- **Description**: Signal handler may not execute if process is killed forcefully
- **Impact**: Stale lock files preventing application restart
- **Fix Required**: Implement more robust cleanup mechanism

### ISSUE-009: Mixed Responsibilities in Server Module
- **File**: [`server.rs`](../src/server.rs)
- **Severity**: MEDIUM
- **Type**: Architecture
- **Description**: Single module handles both gRPC and native messaging concerns
- **Impact**: Difficult to maintain and test
- **Fix Required**: Separate concerns into distinct modules

### ISSUE-010: Synchronous stdio Operations
- **File**: [`server.rs`](../src/server.rs)
- **Lines**: 96-108
- **Severity**: MEDIUM
- **Type**: Performance
- **Description**: Blocking I/O operations in async context
- **Impact**: Poor performance under load
- **Fix Required**: Implement async stdio operations

### ISSUE-011: Inconsistent Error Handling Patterns
- **File**: [`asset_converter.rs`](../src/asset_converter.rs), [`snapshot_converter.rs`](../src/snapshot_converter.rs)
- **Severity**: MEDIUM
- **Type**: Error Handling
- **Description**: Different error handling approaches across converters
- **Impact**: Inconsistent behavior and maintenance overhead
- **Fix Required**: Standardize error handling patterns

### ISSUE-012: Missing Documentation
- **File**: All modules
- **Severity**: MEDIUM
- **Type**: Documentation
- **Description**: Insufficient documentation for public APIs and error conditions
- **Impact**: Difficult maintenance and onboarding
- **Fix Required**: Add comprehensive documentation

## Low Priority Issues

### ISSUE-013: Hardcoded Sentry DSN
- **File**: [`main.rs`](../src/main.rs)
- **Lines**: 154-160
- **Severity**: LOW
- **Type**: Security/Configuration
- **Description**: Sentry DSN hardcoded in source code
- **Impact**: Potential credential exposure
- **Fix Required**: Move to configuration file or environment variable

### ISSUE-014: Inefficient JSON Parsing
- **File**: All converter modules
- **Severity**: LOW
- **Type**: Performance
- **Description**: Multiple JSON parsing passes for same data
- **Impact**: Unnecessary CPU overhead
- **Fix Required**: Optimize parsing to single pass where possible

### ISSUE-015: Missing Cargo.toml Edition Update
- **File**: [`Cargo.toml`](../Cargo.toml)
- **Line**: 4
- **Severity**: LOW
- **Type**: Configuration
- **Description**: Using Rust edition "2024" which doesn't exist (should be "2021")
- **Impact**: Potential compilation issues
- **Fix Required**: Update to valid Rust edition

### ISSUE-016: Unused Dependencies
- **File**: [`Cargo.toml`](../Cargo.toml)
- **Severity**: LOW
- **Type**: Dependencies
- **Description**: Some dependencies may not be actively used
- **Impact**: Increased binary size and attack surface
- **Fix Required**: Audit and remove unused dependencies

### ISSUE-017: Missing Error Context
- **File**: All modules
- **Severity**: LOW
- **Type**: Error Handling
- **Description**: Error messages lack sufficient context for debugging
- **Impact**: Difficult troubleshooting
- **Fix Required**: Add contextual error information

### ISSUE-018: No Metrics or Monitoring
- **File**: Entire crate
- **Severity**: LOW
- **Type**: Observability
- **Description**: No metrics collection for monitoring performance or errors
- **Impact**: Limited visibility into production issues
- **Fix Required**: Add metrics collection and monitoring

## Technical Debt Issues

### ISSUE-019: Commented Code in asset_context.rs
- **File**: [`asset_context.rs`](../src/asset_context.rs)
- **Lines**: 126-140
- **Severity**: LOW
- **Type**: Code Quality
- **Description**: Large block of commented-out code
- **Impact**: Code clutter and confusion
- **Fix Required**: Remove commented code or convert to proper documentation

### ISSUE-020: Magic Numbers in Protocol
- **File**: [`server.rs`](../src/server.rs)
- **Lines**: 60-61, 195
- **Severity**: LOW
- **Type**: Code Quality
- **Description**: Magic numbers for buffer sizes without explanation
- **Impact**: Difficult to tune and understand
- **Fix Required**: Extract to named constants with documentation

### ISSUE-021: Inconsistent Naming Conventions
- **File**: Multiple files
- **Severity**: LOW
- **Type**: Code Quality
- **Description**: Inconsistent naming between snake_case and camelCase
- **Impact**: Confusion and maintenance overhead
- **Fix Required**: Standardize naming conventions

## Summary

- **Total Issues**: 21
- **High Priority**: 5
- **Medium Priority**: 7
- **Low Priority**: 9

## Recommended Fix Order

1. **Phase 1 (Critical)**: Issues 001-005 (Error handling and testing)
2. **Phase 2 (Stability)**: Issues 006-012 (Architecture and performance)
3. **Phase 3 (Quality)**: Issues 013-021 (Security and technical debt)

Each issue should be tracked separately with detailed implementation plans and acceptance criteria.