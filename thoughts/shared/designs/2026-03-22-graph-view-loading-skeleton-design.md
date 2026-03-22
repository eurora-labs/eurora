---
date: 2026-03-22
topic: 'Graph View Loading State Fix'
status: validated
---

## Problem Statement

When switching to a previously unloaded thread while in graph view, no loading state is displayed - the user sees nothing. In contrast, list view correctly shows skeletons during loading.

## Constraints

- Must not break existing list view behavior
- Must leverage existing MessageGraph skeleton support
- Must handle both message loading and tree loading states

## Approach

Fix the conditional rendering logic to allow the graph view to render during loading, and combine loading states for proper skeleton display.

**Why this approach:** The MessageGraph component already has skeleton support built-in. We just need to let it render during loading states.

## Root Cause

In `+page.svelte` line 327:

```svelte
{#if messageService.viewMode === 'graph' && messages.length > 0}
```

This condition prevents the graph from rendering when `messages.length === 0`, even during loading. The MessageGraph component has skeleton logic that never executes because the component isn't rendered.

## Architecture

### Current Flow

1. User clicks thread in sidebar while in graph view
2. `getThread(threadId)` creates new `ThreadMessages` with `loading = true`
3. Messages array is empty (async fetch pending)
4. Condition `messages.length > 0` is false
5. Graph branch skipped entirely
6. Falls through to list view, but viewMode is 'graph', causing mismatch

### Fixed Flow

1. User clicks thread in sidebar while in graph view
2. `getThread(threadId)` creates new `ThreadMessages` with `loading = true`
3. Messages array is empty but `messagesLoading` is true
4. Condition `messages.length > 0 || messagesLoading` is true
5. MessageGraph renders with `loading=true`
6. MessageGraph shows skeleton nodes via `addSkeletonPath()`
7. When messages load, graph updates with real data

## Components

### Modified: `+page.svelte`

- Update graph view condition to include loading state
- Pass combined loading state to MessageGraph

### Unchanged: `MessageGraph` (in packages/ui)

- Already has skeleton support via `addSkeletonPath()`
- Already handles `loading` prop correctly

## Data Flow

```
Thread Switch (Graph View)
    │
    ├─► getThread() called
    │       │
    │       └─► ensureLoaded() creates entry with loading=true
    │
    ├─► threadData?.loading = true
    │
    ├─► messagesLoading = true
    │
    └─► Graph renders with loading=true
            │
            └─► MessageGraph shows 2 skeleton nodes
                    │
                    └─► When fetch completes: loading=false, real nodes render
```

## Changes Required

### File: `apps/desktop/src/routes/(chat)/[[id]]/+page.svelte`

**Change 1: Line 327 - Update condition**

```diff
- {#if messageService.viewMode === 'graph' && messages.length > 0}
+ {#if messageService.viewMode === 'graph' && (messages.length > 0 || messagesLoading)}
```

**Change 2: Line 333 - Update loading prop**

```diff
  <MessageGraph
    {treeNodes}
    {activeMessageIds}
    startLabel={threadTitle}
-   loading={treeLoading}
+   loading={messagesLoading || treeLoading}
    hasMoreLevels={treeHasMore}
    loadingMoreLevels={treeLoading}
    onmessagedblclick={handleGraphNodeDblClick}
    onloadmorelevels={handleLoadMoreLevels}
  />
```

## Testing Strategy

1. **Test case 1:** Switch to graph view, click on previously unloaded thread
    - Expected: Skeleton nodes appear briefly, then real messages

2. **Test case 2:** Switch to graph view, click on already loaded thread
    - Expected: Graph renders immediately with real data (no regression)

3. **Test case 3:** Switch to list view, click on previously unloaded thread
    - Expected: List view skeletons appear (no regression)

4. **Test case 4:** Create new thread while in graph view
    - Expected: Empty state/suggestions show correctly

## Open Questions

None - straightforward fix.
