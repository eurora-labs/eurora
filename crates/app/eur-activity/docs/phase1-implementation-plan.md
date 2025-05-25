# Phase 1 Implementation Plan - eur-activity Crate

## Overview
This document outlines the specific implementation plan for Phase 1 critical fixes to the `eur-activity` crate. This phase focuses on eliminating runtime panics and implementing core missing functionality.

## Estimated Timeline: 1-2 weeks

## Task Breakdown

### Task 1: Implement Timestamp Tracking (Priority: CRITICAL)
**Estimated Effort:** 2-3 days  
**Files:** [`browser_activity.rs`](../src/browser_activity.rs)

#### 1.1 Add Timestamp Fields to Snapshot Structs
- [ ] Add `created_at: u64` field to [`ArticleSnapshot`](../src/browser_activity.rs:172)
- [ ] Add `updated_at: u64` field to [`ArticleSnapshot`](../src/browser_activity.rs:172)
- [ ] Add `created_at: u64` field to [`YoutubeSnapshot`](../src/browser_activity.rs:197)
- [ ] Add `updated_at: u64` field to [`YoutubeSnapshot`](../src/browser_activity.rs:197)

#### 1.2 Update Constructors
- [ ] Modify [`ArticleSnapshot`] constructor to set timestamps using `chrono::Utc::now().timestamp() as u64`
- [ ] Modify [`YoutubeSnapshot::from()`](../src/browser_activity.rs:201) to set timestamps
- [ ] Consider adding `updated_at` parameter to allow external timestamp setting

#### 1.3 Implement Timestamp Methods
- [ ] Replace [`todo!()`](../src/browser_activity.rs:189) in `ArticleSnapshot::get_updated_at()`
- [ ] Replace [`todo!()`](../src/browser_activity.rs:192) in `ArticleSnapshot::get_created_at()`
- [ ] Replace [`todo!()`](../src/browser_activity.rs:242) in `YoutubeSnapshot::get_updated_at()`
- [ ] Replace [`todo!()`](../src/browser_activity.rs:246) in `YoutubeSnapshot::get_created_at()`

### Task 2: Implement State Gathering (Priority: CRITICAL)
**Estimated Effort:** 1-2 days  
**Files:** [`browser_activity.rs`](../src/browser_activity.rs:389)

#### 2.1 Design State Representation
- [ ] Define JSON structure for browser state representation
- [ ] Include activity metadata (process name, start time, asset count)
- [ ] Include current browser state summary

#### 2.2 Implement gather_state() Method
- [ ] Replace [`todo!()`](../src/browser_activity.rs:389) in `BrowserStrategy::gather_state()`
- [ ] Return JSON string with current activity state
- [ ] Include error handling for serialization failures

#### 2.3 Example Implementation Structure
```rust
fn gather_state(&self) -> String {
    let state = json!({
        "process_name": self.process_name,
        "name": self.name,
        "timestamp": Utc::now().timestamp(),
        "status": "active",
        "asset_count": 0, // Will be updated when assets are available
    });
    state.to_string()
}
```

### Task 3: Replace Panic-Prone Error Handling (Priority: CRITICAL)
**Estimated Effort:** 3-4 days  
**Files:** [`browser_activity.rs`](../src/browser_activity.rs)

#### 3.1 Create Custom Error Types
- [ ] Define `ActivityError` enum in new `error.rs` module
- [ ] Add variants for `ImageProcessing`, `ProtocolBuffer`, `NetworkTimeout`
- [ ] Implement `From` traits for common error types (`image::ImageError`, `tonic::Status`)

#### 3.2 Replace Image Loading Panics
- [ ] Replace [`expect("Failed to load PNG image from proto")`](../src/browser_activity.rs:65) with proper error handling
- [ ] Replace [`expect("Failed to load JPEG image from proto")`](../src/browser_activity.rs:69) with proper error handling
- [ ] Replace [`expect("Failed to load WebP image from proto")`](../src/browser_activity.rs:75) with proper error handling
- [ ] Add similar fixes in [`YoutubeSnapshot::from()`](../src/browser_activity.rs:201) method

#### 3.3 Handle Protocol Buffer Validation
- [ ] Replace [`unwrap()`](../src/browser_activity.rs:58) on `state.video_frame` with proper validation
- [ ] Add validation for required protocol buffer fields
- [ ] Implement graceful fallbacks for missing data

#### 3.4 Update Method Signatures
- [ ] Change `YoutubeAsset::from()` to return `Result<YoutubeAsset, ActivityError>`
- [ ] Change `YoutubeSnapshot::from()` to return `Result<YoutubeSnapshot, ActivityError>`
- [ ] Update calling code to handle new error types

### Task 4: Add Basic Unit Tests (Priority: HIGH)
**Estimated Effort:** 2-3 days  
**Files:** [`lib.rs`](../src/lib.rs:200), new test files

#### 4.1 Test Infrastructure Setup
- [ ] Replace empty [`mod tests {}`](../src/lib.rs:200) with actual test implementations
- [ ] Add test dependencies to `Cargo.toml` (`tokio-test`, `tempfile`)
- [ ] Create test fixtures for protocol buffer data

#### 4.2 Core Functionality Tests
- [ ] Test `Activity::new()` with various parameters
- [ ] Test `Activity::get_display_assets()` method
- [ ] Test `Activity::get_context_chips()` method
- [ ] Test `select_strategy_for_process()` function

#### 4.3 Error Handling Tests
- [ ] Test image loading with invalid data
- [ ] Test protocol buffer conversion with missing fields
- [ ] Test timestamp generation and retrieval

#### 4.4 Strategy Tests
- [ ] Test `BrowserStrategy::new()` creation
- [ ] Test `DefaultStrategy` fallback behavior
- [ ] Mock gRPC client for isolated testing

### Task 5: Documentation and Code Comments (Priority: MEDIUM)
**Estimated Effort:** 1 day  
**Files:** All source files

#### 5.1 Add Method Documentation
- [ ] Add rustdoc comments to all public methods
- [ ] Document error conditions and return types
- [ ] Add usage examples for key functions

#### 5.2 Explain Extension ID Design
- [ ] Add comments explaining hardcoded extension IDs
- [ ] Document asset processing pipeline identification
- [ ] Add module-level documentation

## Implementation Order

### Week 1
1. **Days 1-2:** Task 1 (Timestamp Tracking)
2. **Days 3-4:** Task 2 (State Gathering)
3. **Day 5:** Start Task 3 (Error Types Definition)

### Week 2
1. **Days 1-3:** Complete Task 3 (Error Handling)
2. **Days 4-5:** Task 4 (Unit Tests)
3. **Weekend:** Task 5 (Documentation)

## Success Criteria

### Functional Requirements
- [ ] All `todo!()` methods implemented and functional
- [ ] No runtime panics in normal operation paths
- [ ] Proper error handling with `Result` types
- [ ] Basic test coverage for core functionality

### Quality Requirements
- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Documentation updated for changed APIs
- [ ] Error messages are informative and actionable

## Risk Mitigation

### Technical Risks
- **Protocol Buffer Changes:** Keep backward compatibility, add validation
- **Image Processing Complexity:** Start with basic error handling, optimize later
- **Test Data Creation:** Use simple fixtures initially, expand as needed

### Timeline Risks
- **Scope Creep:** Focus only on eliminating panics and basic functionality
- **Testing Complexity:** Prioritize happy path tests, add edge cases later
- **Documentation Overhead:** Keep initial docs minimal but accurate

## Dependencies and Blockers

### External Dependencies
- No new external dependencies required for Phase 1
- Existing dependencies are sufficient for basic implementation

### Internal Dependencies
- May need coordination with `eur-proto` team if protocol buffer changes are needed
- No blocking dependencies identified

## Testing Strategy for Phase 1

### Unit Tests
- Focus on individual method functionality
- Mock external dependencies (gRPC clients)
- Test error conditions with invalid inputs

### Integration Tests
- Basic end-to-end activity creation
- Simple asset collection scenarios
- Error propagation through the system

### Manual Testing
- Verify no runtime panics with real browser data
- Test with various image formats and sizes
- Validate timestamp accuracy and consistency

## Deliverables

1. **Updated Source Code**
   - All `todo!()` methods implemented
   - Comprehensive error handling
   - Basic unit test suite

2. **Documentation Updates**
   - Updated rustdoc comments
   - Code comments explaining design decisions
   - Updated README if API changes

3. **Test Results**
   - All tests passing
   - Basic coverage report
   - Manual testing verification

## Next Steps After Phase 1

1. **Phase 2 Preparation**
   - Review performance with real data
   - Identify optimization opportunities
   - Plan advanced error handling features

2. **Feedback Integration**
   - Gather feedback from other team members
   - Identify additional test scenarios
   - Plan integration with other crates

---

**Note:** This plan focuses on eliminating critical runtime issues while maintaining the existing architectural design. More advanced features and optimizations are planned for subsequent phases.