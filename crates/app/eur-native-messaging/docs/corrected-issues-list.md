# Corrected Issues List: eur-native-messaging Crate

## Critical Issues (Immediate Attention Required)

### ISSUE-001: Background Script Message Routing Failure
- **File**: [`native-messaging-worker.ts`](../../../apps/extension/background-script/src/lib/service-worker/native-messaging-worker.ts)
- **Lines**: 120-152
- **Severity**: CRITICAL
- **Type**: Communication Protocol
- **Description**: Message listener attached to `nativePort.onMessage` instead of `chrome.runtime.onMessage`
- **Impact**: Content script messages never reach the background script handler
- **Current Code**:
  ```typescript
  nativePort.onMessage.addListener(async (message, sender) => {
      switch (message.type) {
          case 'GENERATE_ASSETS':
  ```
- **Should Be**:
  ```typescript
  chrome.runtime.onMessage.addListener(async (message, sender, sendResponse) => {
      switch (message.type) {
          case 'GENERATE_ASSETS':
  ```
- **Fix Required**: Correct event listener attachment and implement proper message routing

### ISSUE-002: PDF Watcher Protocol Mismatch
- **File**: [`pdf-watcher.ts`](../../../apps/extension/content-scripts/pdf-watcher/src/lib/pdf-watcher.ts)
- **Line**: 16
- **Severity**: CRITICAL
- **Type**: Protocol Inconsistency
- **Description**: PDF watcher listens for `GENERATE_PDF_REPORT` while other watchers use `GENERATE_ASSETS`
- **Impact**: PDF functionality completely broken - messages never received
- **Fix Required**: Change message type to `GENERATE_ASSETS` to match other content scripts

### ISSUE-003: Empty Snapshot Handler Implementation
- **File**: [`native-messaging-worker.ts`](../../../apps/extension/background-script/src/lib/service-worker/native-messaging-worker.ts)
- **Line**: 158
- **Severity**: CRITICAL
- **Type**: Missing Implementation
- **Description**: `handleGenerateSnapshot()` function is empty
- **Impact**: Snapshot functionality doesn't work
- **Fix Required**: Implement snapshot generation logic similar to `handleGenerateReport()`

### ISSUE-004: Protocol Field Naming Inconsistency
- **File**: [`tauri_ipc.proto`](../../../proto/tauri_ipc.proto) line 68 vs [`asset_context.rs`](../src/asset_context.rs) line 152
- **Severity**: CRITICAL
- **Type**: Protocol Definition
- **Description**: Field named `selectedText` in proto but accessed as `selected_text` in Rust
- **Impact**: Data loss during PDF state conversion
- **Fix Required**: Standardize field naming across protocol definitions

## High Priority Issues

### ISSUE-005: Hardcoded Message Format in Background Script
- **File**: [`native-messaging-worker.ts`](../../../apps/extension/background-script/src/lib/service-worker/native-messaging-worker.ts)
- **Lines**: 77-84
- **Severity**: HIGH
- **Type**: Protocol Violation
- **Description**: Hardcoded `TRANSCRIPT` message format doesn't match protocol definitions
- **Current Code**:
  ```typescript
  const nativeMessage = {
      type: 'TRANSCRIPT',
      videoId: payload.videoId || 'unknown',
      transcript: typeof payload.transcript === 'string' 
          ? payload.transcript 
          : JSON.stringify(payload.transcript)
  };
  ```
- **Impact**: Native host receives unexpected message format
- **Fix Required**: Use protocol-defined message structures from generated types

### ISSUE-006: Base64 Decoding Without Error Handling
- **File**: [`asset_context.rs`](../src/asset_context.rs) lines 56-58, [`snapshot_context.rs`](../src/snapshot_context.rs) lines 33-35
- **Severity**: HIGH
- **Type**: Error Handling
- **Description**: Base64 decoding uses `unwrap()` without handling malformed data
- **Impact**: Runtime panics if extension sends corrupted image data
- **Fix Required**: Implement proper error handling for decode operations

### ISSUE-007: Stdio Concurrency Issues
- **File**: [`server.rs`](../src/server.rs)
- **Lines**: 93-94
- **Severity**: HIGH
- **Type**: Concurrency
- **Description**: Acquiring multiple mutexes simultaneously without proper ordering
- **Impact**: Potential deadlock under concurrent load
- **Fix Required**: Implement proper mutex ordering or use single mutex

### ISSUE-008: Missing Error Context in Converters
- **File**: [`asset_converter.rs`](../src/asset_converter.rs) lines 17-20
- **Severity**: HIGH
- **Type**: Error Handling
- **Description**: Generic error messages without context about what failed
- **Impact**: Difficult debugging when conversion fails
- **Fix Required**: Add contextual error information with field names and values

## Medium Priority Issues

### ISSUE-009: Background Script Connection Logic
- **File**: [`native-messaging-worker.ts`](../../../apps/extension/background-script/src/lib/service-worker/native-messaging-worker.ts)
- **Lines**: 33-44, 53-58
- **Severity**: MEDIUM
- **Type**: Connection Management
- **Description**: Reconnection logic may cause infinite loops or resource leaks
- **Impact**: Poor connection reliability
- **Fix Required**: Implement exponential backoff and connection state management

### ISSUE-010: Inconsistent Message Type Handling
- **File**: [`asset_converter.rs`](../src/asset_converter.rs) lines 22-48
- **Severity**: MEDIUM
- **Type**: Architecture
- **Description**: Different message types use different conversion patterns
- **Impact**: Maintenance overhead and potential bugs
- **Fix Required**: Standardize conversion patterns across all message types

### ISSUE-011: Missing Integration Tests
- **File**: Entire communication pipeline
- **Severity**: MEDIUM
- **Type**: Testing
- **Description**: No tests for extension-to-native communication flow
- **Impact**: Undetected integration issues
- **Fix Required**: Add end-to-end tests for complete communication pipeline

### ISSUE-012: Unused Background Script Code
- **File**: [`background.ts`](../../../apps/extension/background-script/src/lib/background.ts)
- **Lines**: 1-32
- **Severity**: MEDIUM
- **Type**: Code Quality
- **Description**: Entire strategy pattern implementation is commented out
- **Impact**: Code confusion and maintenance overhead
- **Fix Required**: Remove commented code or implement the strategy pattern

## Low Priority Issues

### ISSUE-013: Hardcoded Sentry DSN
- **File**: [`main.rs`](../src/main.rs)
- **Lines**: 154-160
- **Severity**: LOW
- **Type**: Security/Configuration
- **Description**: Sentry DSN hardcoded in source code
- **Impact**: Potential credential exposure
- **Fix Required**: Move to configuration file or environment variable

### ISSUE-014: Missing Documentation
- **File**: All modules
- **Severity**: LOW
- **Type**: Documentation
- **Description**: Insufficient documentation for communication protocol
- **Impact**: Difficult maintenance and onboarding
- **Fix Required**: Document the complete extension-to-native communication flow

### ISSUE-015: Cargo.toml Edition Issue
- **File**: [`Cargo.toml`](../Cargo.toml)
- **Line**: 4
- **Severity**: LOW
- **Type**: Configuration
- **Description**: Using Rust edition "2024" which doesn't exist
- **Impact**: Potential compilation issues
- **Fix Required**: Update to "2021" edition

### ISSUE-016: Magic Numbers in Buffer Sizes
- **File**: [`server.rs`](../src/server.rs)
- **Lines**: 60-61, 195
- **Severity**: LOW
- **Type**: Code Quality
- **Description**: Magic numbers for buffer sizes without explanation
- **Impact**: Difficult to tune and understand
- **Fix Required**: Extract to named constants with documentation

## Extension-Specific Issues

### ISSUE-017: YouTube Watcher Error Handling
- **File**: [`youtube-watcher.ts`](../../../apps/extension/content-scripts/youtube-watcher/src/lib/youtube-watcher.ts)
- **Lines**: 138-141
- **Severity**: MEDIUM
- **Type**: Error Handling
- **Description**: Generic error response without proper error propagation
- **Impact**: Difficult debugging of YouTube-specific issues
- **Fix Required**: Implement proper error context and logging

### ISSUE-018: Article Watcher Unused Code
- **File**: [`article-watcher.ts`](../../../apps/extension/content-scripts/article-watcher/src/lib/article-watcher.ts)
- **Lines**: 66-104
- **Severity**: LOW
- **Type**: Code Quality
- **Description**: `extractArticleContent()` function defined but never used
- **Impact**: Code clutter
- **Fix Required**: Remove unused function or integrate with Readability usage

### ISSUE-019: PDF Watcher Error Handling
- **File**: [`pdf-watcher.ts`](../../../apps/extension/content-scripts/pdf-watcher/src/lib/pdf-watcher.ts)
- **Lines**: 32-34
- **Severity**: MEDIUM
- **Type**: Error Handling
- **Description**: Throws error without proper error response format
- **Impact**: Inconsistent error handling across content scripts
- **Fix Required**: Standardize error response format

## Summary

- **Total Issues**: 19
- **Critical**: 4 (Communication broken)
- **High Priority**: 4 (Error handling and concurrency)
- **Medium Priority**: 7 (Architecture and testing)
- **Low Priority**: 4 (Quality and configuration)

## Fix Priority Order

### Phase 1 (Critical - Week 1)
1. Fix background script message routing (ISSUE-001)
2. Standardize PDF watcher protocol (ISSUE-002)
3. Implement snapshot handler (ISSUE-003)
4. Fix protocol field naming (ISSUE-004)

### Phase 2 (High Priority - Week 2)
1. Fix hardcoded message format (ISSUE-005)
2. Add base64 error handling (ISSUE-006)
3. Resolve stdio concurrency (ISSUE-007)
4. Add error context (ISSUE-008)

### Phase 3 (Medium Priority - Week 3)
1. Improve connection management (ISSUE-009)
2. Standardize conversion patterns (ISSUE-010)
3. Add integration tests (ISSUE-011)
4. Clean up unused code (ISSUE-012)

### Phase 4 (Quality - Week 4)
1. Configuration management (ISSUE-013)
2. Documentation (ISSUE-014)
3. Code quality improvements (ISSUE-015, ISSUE-016)
4. Extension error handling (ISSUE-017, ISSUE-018, ISSUE-019)

The critical issues must be fixed first as they prevent the extension from communicating with the native host at all.