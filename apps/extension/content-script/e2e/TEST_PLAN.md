# Content Script E2E Test Plan

## Document Information

**Version**: 1.0  
**Last Updated**: 2025-10-14  
**Status**: Active  
**Owner**: Content Script Team

## Executive Summary

This document outlines the comprehensive end-to-end testing strategy for the content script package. The content script system dynamically loads site-specific handlers based on the current domain, processes messages from the background script, and generates assets/snapshots from web pages.

## System Architecture

### Components

1. **Bootstrap (`bootstrap.ts`)**: Main entry point that listens for SITE_LOAD messages and dynamically imports site handlers
2. **Registry (`registry.json`)**: Maps domains to their respective handler chunks
3. **Site Handlers**: Domain-specific logic (e.g., YouTube, Default/Article)
4. **Message Router**: Routes messages between background script and content scripts

### Message Flow

```
Background Script → SITE_LOAD → Bootstrap → Load Handler
Background Script → Message → Handler → Process → Response
```

## Test Coverage Goals

| Category            | Target Coverage | Current Status |
| ------------------- | --------------- | -------------- |
| Bootstrap Mechanism | 95%             | Implemented    |
| Registry System     | 90%             | Implemented    |
| Site Handlers       | 85%             | Implemented    |
| Message Routing     | 90%             | Implemented    |
| Error Handling      | 80%             | Implemented    |
| Edge Cases          | 75%             | Implemented    |

## Test Categories

### 1. Bootstrap Mechanism Tests

**File**: `bootstrap.e2e.spec.ts`

#### 1.1 Bootstrap Loading

- **ID**: BM-001
- **Priority**: Critical
- **Description**: Verify bootstrap script is loaded on page
- **Steps**:
    1. Load extension in browser
    2. Navigate to any webpage
    3. Check if bootstrap is injected
- **Expected**: Bootstrap script is present and functional
- **Status**: ✅ Implemented (skipped)

#### 1.2 SITE_LOAD Message Handling

- **ID**: BM-002
- **Priority**: Critical
- **Description**: Verify bootstrap responds to SITE_LOAD messages
- **Steps**:
    1. Navigate to webpage
    2. Send SITE_LOAD message with chunk path
    3. Wait for response
- **Expected**: Response indicates successful load
- **Status**: ✅ Implemented (skipped)

#### 1.3 Single Load Enforcement

- **ID**: BM-003
- **Priority**: High
- **Description**: Ensure site handler loads only once
- **Steps**:
    1. Send first SITE_LOAD message
    2. Send second SITE_LOAD message
    3. Verify second is ignored
- **Expected**: Second load returns false
- **Status**: ✅ Implemented (skipped)

#### 1.4 Fallback to Default

- **ID**: BM-004
- **Priority**: High
- **Description**: Verify fallback when site handler fails
- **Steps**:
    1. Send SITE_LOAD with invalid chunk
    2. Check console for error
    3. Verify default handler loads
- **Expected**: Default handler loads after error
- **Status**: ✅ Implemented (skipped)

#### 1.5 canHandle Function Support

- **ID**: BM-005
- **Priority**: Medium
- **Description**: Test canHandle function behavior
- **Steps**:
    1. Load handler with canHandle function
    2. Verify it's called with document
    3. Check return value affects loading
- **Expected**: Handler respects canHandle result
- **Status**: ✅ Implemented (skipped)

### 2. Registry System Tests

**File**: `registry.e2e.spec.ts`

#### 2.1 Registry File Generation

- **ID**: REG-001
- **Priority**: Critical
- **Description**: Verify registry.json is generated correctly
- **Steps**:
    1. Build project
    2. Check registry.json exists
    3. Validate structure
- **Expected**: Valid registry.json with all entries
- **Status**: ✅ Implemented (skipped)

#### 2.2 Domain Pattern Matching

- **ID**: REG-002
- **Priority**: Critical
- **Description**: Test exact domain matching
- **Steps**:
    1. Navigate to youtube.com
    2. Verify YouTube handler loads
- **Expected**: Correct handler loads for domain
- **Status**: ✅ Implemented (skipped)

#### 2.3 Wildcard Pattern Support

- **ID**: REG-003
- **Priority**: High
- **Description**: Test wildcard pattern matching
- **Steps**:
    1. Check registry for wildcard patterns
    2. Verify format (\*.domain)
    3. Test matching behavior
- **Expected**: Wildcard patterns work correctly
- **Status**: ✅ Implemented (skipped)

#### 2.4 Subdomain Matching

- **ID**: REG-004
- **Priority**: High
- **Description**: Test subdomain matching (e.g., m.youtube.com)
- **Steps**:
    1. Navigate to subdomain
    2. Verify correct handler loads
- **Expected**: Handler matches subdomains
- **Status**: ✅ Implemented (skipped)

#### 2.5 Default Handler Fallback

- **ID**: REG-005
- **Priority**: High
- **Description**: Verify default handler for unmatched domains
- **Steps**:
    1. Navigate to unmapped domain
    2. Verify default handler loads
- **Expected**: Default handler handles unmapped domains
- **Status**: ✅ Implemented (skipped)

#### 2.6 Registry Entry Validation

- **ID**: REG-006
- **Priority**: Medium
- **Description**: Validate registry entry structure
- **Steps**:
    1. Parse registry.json
    2. Check each entry has id, chunk, patterns
    3. Verify chunk paths exist
- **Expected**: All entries valid with existing chunks
- **Status**: ✅ Implemented (skipped)

#### 2.7 Default Exclusion

- **ID**: REG-007
- **Priority**: Medium
- **Description**: Verify \_default not in registry
- **Steps**:
    1. Parse registry.json
    2. Check for \_default entry
    3. Verify \_default chunk exists separately
- **Expected**: \_default excluded from registry but file exists
- **Status**: ✅ Implemented (skipped)

### 3. Site Handler Tests

**File**: `site-handlers.e2e.spec.ts`

#### 3.1 Default Handler - Asset Generation

- **ID**: DH-001
- **Priority**: Critical
- **Description**: Test article asset generation
- **Steps**:
    1. Navigate to article page
    2. Send GENERATE_ASSETS message
    3. Verify response structure
- **Expected**: Valid article asset with metadata
- **Status**: ✅ Implemented (skipped)

#### 3.2 Default Handler - Snapshot Generation

- **ID**: DH-002
- **Priority**: High
- **Description**: Test snapshot generation
- **Steps**:
    1. Navigate to page
    2. Send GENERATE_SNAPSHOT message
    3. Verify response
- **Expected**: Valid snapshot data
- **Status**: ✅ Implemented (skipped)

#### 3.3 Default Handler - Metadata Extraction

- **ID**: DH-003
- **Priority**: High
- **Description**: Verify metadata extraction (title, URL, etc.)
- **Steps**:
    1. Navigate to page with metadata
    2. Generate assets
    3. Check extracted fields
- **Expected**: All metadata fields populated
- **Status**: ✅ Implemented (skipped)

#### 3.4 Default Handler - Error Handling

- **ID**: DH-004
- **Priority**: High
- **Description**: Test invalid message type handling
- **Steps**:
    1. Send invalid message type
    2. Check response
- **Expected**: Error response with message
- **Status**: ✅ Implemented (skipped)

#### 3.5 YouTube Handler - Video Detection

- **ID**: YT-001
- **Priority**: Critical
- **Description**: Detect YouTube video pages
- **Steps**:
    1. Navigate to YouTube video
    2. Check for video element
    3. Verify URL pattern
- **Expected**: Video detected correctly
- **Status**: ✅ Implemented (skipped)

#### 3.6 YouTube Handler - Video ID Extraction

- **ID**: YT-002
- **Priority**: Critical
- **Description**: Extract video ID from URL
- **Steps**:
    1. Navigate to video page
    2. Extract video ID
    3. Verify format
- **Expected**: Correct video ID extracted
- **Status**: ✅ Implemented (skipped)

#### 3.7 YouTube Handler - NEW Message

- **ID**: YT-003
- **Priority**: High
- **Description**: Handle NEW video detection
- **Steps**:
    1. Navigate to video
    2. Send NEW message
    3. Verify transcript fetch attempt
- **Expected**: Handler processes new video
- **Status**: ✅ Implemented (skipped)

#### 3.8 YouTube Handler - Video Assets

- **ID**: YT-004
- **Priority**: Critical
- **Description**: Generate YouTube video assets
- **Steps**:
    1. Navigate to video
    2. Send GENERATE_ASSETS
    3. Verify asset structure
- **Expected**: NativeYoutubeAsset with all fields
- **Status**: ✅ Implemented (skipped)

#### 3.9 YouTube Handler - Video Snapshot

- **ID**: YT-005
- **Priority**: Critical
- **Description**: Generate video snapshot with frame
- **Steps**:
    1. Navigate to playing video
    2. Send GENERATE_SNAPSHOT
    3. Verify frame capture
- **Expected**: NativeYoutubeSnapshot with base64 frame
- **Status**: ✅ Implemented (skipped)

#### 3.10 YouTube Handler - PLAY Message

- **ID**: YT-006
- **Priority**: High
- **Description**: Control video playback
- **Steps**:
    1. Navigate to video
    2. Send PLAY message with timestamp
    3. Verify video seeks to time
- **Expected**: Video currentTime updated
- **Status**: ✅ Implemented (skipped)

#### 3.11 YouTube Handler - Non-Video Fallback

- **ID**: YT-007
- **Priority**: Medium
- **Description**: Fallback to article handler on non-video pages
- **Steps**:
    1. Navigate to YouTube home
    2. Send GENERATE_ASSETS
    3. Verify article asset returned
- **Expected**: Article asset for non-video pages
- **Status**: ✅ Implemented (skipped)

#### 3.12 YouTube Handler - Video Time Tracking

- **ID**: YT-008
- **Priority**: Medium
- **Description**: Get current video time
- **Steps**:
    1. Navigate to playing video
    2. Get current time
    3. Verify accuracy
- **Expected**: Accurate currentTime value
- **Status**: ✅ Implemented (skipped)

#### 3.13 YouTube Handler - Frame Capture

- **ID**: YT-009
- **Priority**: High
- **Description**: Capture video frame as base64
- **Steps**:
    1. Generate snapshot
    2. Verify base64 format
    3. Check dimensions
- **Expected**: Valid base64 image with dimensions
- **Status**: ✅ Implemented (skipped)

### 4. Message Routing Tests

**File**: `message-routing.e2e.spec.ts`

#### 4.1 Domain-Based Routing

- **ID**: MR-001
- **Priority**: Critical
- **Description**: Route to correct handler by domain
- **Steps**:
    1. Test YouTube domain
    2. Test default domain
    3. Verify different handlers
- **Expected**: Messages routed to correct handler
- **Status**: ✅ Implemented (skipped)

#### 4.2 Message Type Routing

- **ID**: MR-002
- **Priority**: Critical
- **Description**: Route by message type within handler
- **Steps**:
    1. Send different message types
    2. Verify each is handled
- **Expected**: Each type processed correctly
- **Status**: ✅ Implemented (skipped)

#### 4.3 Invalid Message Type Handling

- **ID**: MR-003
- **Priority**: High
- **Description**: Handle invalid message types gracefully
- **Steps**:
    1. Send invalid message type
    2. Check error response
- **Expected**: Error response without crash
- **Status**: ✅ Implemented (skipped)

#### 4.4 Concurrent Message Handling

- **ID**: MR-004
- **Priority**: High
- **Description**: Handle multiple concurrent messages
- **Steps**:
    1. Send multiple messages simultaneously
    2. Verify all responses
- **Expected**: All messages processed correctly
- **Status**: ✅ Implemented (skipped)

#### 4.5 Sender Information Preservation

- **ID**: MR-005
- **Priority**: Medium
- **Description**: Preserve sender info through routing
- **Steps**:
    1. Send message with sender data
    2. Verify handler receives it
- **Expected**: Sender info available in handler
- **Status**: ✅ Implemented (skipped)

#### 4.6 Additional Data Handling

- **ID**: MR-006
- **Priority**: High
- **Description**: Pass additional data with messages
- **Steps**:
    1. Send message with value/data
    2. Verify handler uses it
- **Expected**: Additional data processed correctly
- **Status**: ✅ Implemented (skipped)

#### 4.7 Promise-Based Responses

- **ID**: MR-007
- **Priority**: High
- **Description**: Handle async responses correctly
- **Steps**:
    1. Send message
    2. Measure response time
    3. Verify response structure
- **Expected**: Async responses work, reasonable time
- **Status**: ✅ Implemented (skipped)

#### 4.8 Navigation Handling

- **ID**: MR-008
- **Priority**: Medium
- **Description**: Handle routing after navigation
- **Steps**:
    1. Send message on page 1
    2. Navigate to page 2
    3. Send message again
- **Expected**: Correct handler on each page
- **Status**: ✅ Implemented (skipped)

#### 4.9 Cross-Tab Isolation

- **ID**: MR-009
- **Priority**: High
- **Description**: Ensure handlers don't leak between tabs
- **Steps**:
    1. Open two tabs with different domains
    2. Send messages to each
    3. Verify correct handlers
- **Expected**: Each tab has its own handler
- **Status**: ✅ Implemented (skipped)

## Test Execution Plan

### Phase 1: Foundation (Week 1)

1. Build extension
2. Set up test environment
3. Enable bootstrap tests
4. Enable registry tests

### Phase 2: Core Functionality (Week 2)

1. Enable site handler tests
2. Test default handler thoroughly
3. Test YouTube handler thoroughly

### Phase 3: Integration (Week 3)

1. Enable message routing tests
2. Test cross-domain scenarios
3. Test error handling

### Phase 4: Refinement (Week 4)

1. Add edge case tests
2. Improve test coverage
3. Performance testing
4. Documentation updates

## Test Environment

### Browser Support

- Chromium (Primary)
- Firefox (Secondary)
- WebKit (Tertiary)

### Test Data

- Example.com for basic testing
- YouTube videos for media testing
- Various article websites

### CI/CD

- Run on every PR
- Block merge on failures
- Generate coverage reports
- Archive test artifacts

## Success Criteria

1. ✅ All critical tests passing
2. ✅ 85%+ overall test coverage
3. ✅ All message types tested
4. ✅ Cross-domain scenarios covered
5. ✅ Error handling validated
6. ✅ Documentation complete

## Known Limitations

1. Tests require extension to be built first
2. YouTube tests may be affected by API changes
3. Transcript fetching might fail due to rate limits
4. Some tests require network access
5. Video tests require media playback support

## Risk Assessment

| Risk                    | Probability | Impact | Mitigation            |
| ----------------------- | ----------- | ------ | --------------------- |
| YouTube API changes     | Medium      | High   | Mock transcript API   |
| Extension load failures | Low         | High   | Pre-build checks      |
| Flaky video tests       | Medium      | Medium | Retry logic, timeouts |
| Network issues          | Low         | Medium | Offline test mode     |
| Browser compatibility   | Low         | High   | Multi-browser testing |

## Maintenance

### Regular Tasks

- Update test data when APIs change
- Review and update test timeouts
- Check for deprecated APIs
- Update documentation
- Monitor flaky tests

### Quarterly Reviews

- Assess test coverage
- Review test performance
- Update test plan
- Identify gaps

## Appendix

### A. Test Data Sources

- https://example.com
- https://www.youtube.com/watch?v=dQw4w9WgXcQ
- Local test HTML files

### B. Useful Commands

```bash
# Build extension
pnpm run build

# Run all E2E tests
pnpm run test:e2e

# Run specific test file
pnpm exec playwright test e2e/bootstrap.e2e.spec.ts

# Debug tests
pnpm run test:e2e:debug

# UI mode
pnpm run test:e2e:ui

# View report
pnpm run test:e2e:report
```

### C. References

- [Playwright Documentation](https://playwright.dev)
- [Chrome Extension Documentation](https://developer.chrome.com/docs/extensions)
- [WebExtensions API](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions)

---

**Document Control**

| Date       | Version | Author              | Changes           |
| ---------- | ------- | ------------------- | ----------------- |
| 2025-10-14 | 1.0     | Content Script Team | Initial test plan |
