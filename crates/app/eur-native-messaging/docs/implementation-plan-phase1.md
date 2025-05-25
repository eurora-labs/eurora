# Implementation Plan - Phase 1: Critical Communication Fixes

## Overview

This plan addresses the 4 critical issues that are preventing the browser extension from communicating with the native host. These must be fixed first before any other improvements can be made.

## Critical Issues to Fix

### DENIED, this works as expected 1. Background Script Message Routing (ISSUE-001)
**Priority**: CRITICAL
**File**: `apps/extension/background-script/src/lib/service-worker/native-messaging-worker.ts`
**Problem**: Message listener attached to wrong object, content script messages never reach handlers

#### Current Broken Code (lines 120-152):
```typescript
nativePort.onMessage.addListener(async (message, sender) => {
    switch (message.type) {
        case 'GENERATE_ASSETS':
            // This never executes because messages come from chrome.runtime, not nativePort
```

#### Fix Required:
1. Move message listener to `chrome.runtime.onMessage`
2. Implement proper message routing from content scripts to native host
3. Add proper response handling back to content scripts

### 2. PDF Watcher Protocol Mismatch (ISSUE-002)
**Priority**: CRITICAL
**File**: `apps/extension/content-scripts/pdf-watcher/src/lib/pdf-watcher.ts`
**Problem**: Uses `GENERATE_PDF_REPORT` instead of `GENERATE_ASSETS`

#### Current Code (line 16):
```typescript
case 'GENERATE_PDF_REPORT':
```

#### Fix Required:
```typescript
case 'GENERATE_ASSETS':
```

### 3. Empty Snapshot Handler (ISSUE-003)
**Priority**: CRITICAL
**File**: `apps/extension/background-script/src/lib/service-worker/native-messaging-worker.ts`
**Problem**: `handleGenerateSnapshot()` function is empty (line 158)

#### Fix Required:
Implement snapshot generation logic similar to `handleGenerateReport()` but for snapshot requests.

### DENIED 4. Protocol Field Naming Inconsistency (ISSUE-004)
**Priority**: CRITICAL
**File**: `proto/tauri_ipc.proto` vs `crates/app/eur-native-messaging/src/asset_context.rs`
**Problem**: Field named `selectedText` in proto but accessed as `selected_text` in Rust

#### Options:
- **Option A**: Update proto to use `selected_text` (affects TypeScript generation)
- **Option B**: Update Rust code to use `selectedText` (simpler change)
- **Option C**: Use field mapping in Rust (most compatible)

## Implementation Steps

### Step 1: Fix Background Script Message Routing
**Estimated Time**: 2-3 hours
**Files to Modify**: 
- `apps/extension/background-script/src/lib/service-worker/native-messaging-worker.ts`

**Changes**:
1. Remove incorrect message listener from `nativePort`
2. Add proper `chrome.runtime.onMessage` listener
3. Implement message routing logic
4. Add proper error handling and response forwarding

### Step 2: Fix PDF Watcher Protocol
**Estimated Time**: 30 minutes
**Files to Modify**:
- `apps/extension/content-scripts/pdf-watcher/src/lib/pdf-watcher.ts`

**Changes**:
1. Change `GENERATE_PDF_REPORT` to `GENERATE_ASSETS`
2. Test PDF functionality works with corrected message type

### Step 3: Implement Snapshot Handler
**Estimated Time**: 1-2 hours
**Files to Modify**:
- `apps/extension/background-script/src/lib/service-worker/native-messaging-worker.ts`

**Changes**:
1. Implement `handleGenerateSnapshot()` function
2. Add proper error handling
3. Test snapshot functionality

### Step 4: Fix Protocol Field Naming
**Estimated Time**: 1 hour
**Files to Modify** (Option B - simplest):
- `crates/app/eur-native-messaging/src/asset_context.rs`

**Changes**:
1. Update field access from `selected_text` to `selectedText`
2. Test PDF state conversion works correctly

## Testing Plan

### Manual Testing
1. **Extension Loading**: Verify extension loads without errors
2. **YouTube Functionality**: Test asset generation on YouTube videos
3. **Article Functionality**: Test asset generation on article pages  
4. **PDF Functionality**: Test asset generation on PDF pages
5. **Snapshot Functionality**: Test snapshot generation
6. **Native Communication**: Verify messages reach native host

### Integration Testing
1. **End-to-End Flow**: Content script → Background → Native host → Desktop app
2. **Error Scenarios**: Test connection failures, malformed data
3. **Multi-tab**: Test concurrent requests from multiple tabs

## Success Criteria

### Phase 1 Complete When:
- [ ] Content script messages reach background script handlers
- [ ] Background script successfully forwards messages to native host
- [ ] PDF watcher responds to `GENERATE_ASSETS` messages
- [ ] Snapshot generation works for all content types
- [ ] Protocol field naming is consistent
- [ ] All content script types (YouTube, Article, PDF) work end-to-end
- [ ] No console errors in extension or native host logs

## Risk Assessment

### Low Risk Changes:
- PDF watcher message type change (isolated change)
- Protocol field naming fix (simple field access update)

### Medium Risk Changes:
- Background script message routing (core communication logic)
- Snapshot handler implementation (new functionality)

### Mitigation Strategies:
1. **Incremental Testing**: Test each change individually
2. **Rollback Plan**: Keep backup of working code
3. **Logging**: Add comprehensive logging for debugging
4. **Staged Deployment**: Test in development environment first

## Dependencies

### Required Before Starting:
- [ ] Development environment set up
- [ ] Extension build process working
- [ ] Native host compilation working
- [ ] Test browser extension installation process

### External Dependencies:
- Protocol buffer compilation (should be automatic)
- TypeScript compilation for extension
- Rust compilation for native host

## Notes for Implementation

### Background Script Fix Details:
The current code has a fundamental misunderstanding of the Chrome extension message flow:
- Content scripts send messages via `chrome.runtime.sendMessage()`
- These arrive at the background script via `chrome.runtime.onMessage`
- The background script then forwards to native host via `nativePort.postMessage()`
- Responses come back via `nativePort.onMessage` and need forwarding to content scripts

### Message Flow Should Be:
```
Content Script → chrome.runtime.sendMessage() 
    ↓
Background Script chrome.runtime.onMessage listener
    ↓
Background Script nativePort.postMessage()
    ↓
Native Host receives message
    ↓
Native Host sends response
    ↓
Background Script nativePort.onMessage listener
    ↓
Background Script chrome.tabs.sendMessage() back to content script
```

## Post-Phase 1

After these critical fixes are complete:
- Extension should have basic communication working
- All content script types should function
- Ready for Phase 2: Error handling and stability improvements
- Ready for comprehensive testing and optimization

---

**Review and modify this plan as needed before implementation begins.**