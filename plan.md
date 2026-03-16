# Message Tree & Conversation Branching — Implementation Spec

This document describes how LibreChat stores, resolves, and renders a branching conversation tree. It contains everything needed to reimplement this system from scratch.

---

## 1. Data Model

### 1.1 Message Schema

Each message is a flat document in MongoDB. The tree structure is **implicit** — encoded entirely through `parentMessageId` pointers. There is no explicit `children`, `siblingIndex`, or `depth` column in the database.

```
Message {
  messageId:        string   (UUID, unique, indexed)
  conversationId:   string   (indexed, required)
  parentMessageId:  string   (nullable — root messages use the sentinel below)
  user:             string   (owner user ID)
  isCreatedByUser:  boolean
  sender:           string   (display name: "User", "GPT-4", agent name, etc.)
  text:             string   (markdown body)
  content:          Mixed[]  (optional — structured multi-part content for agents/artifacts)
  model:            string
  endpoint:         string
  error:            boolean
  unfinished:       boolean
  createdAt:        Date     (auto, via timestamps: true)
  updatedAt:        Date     (auto)
  ...               (tokenCount, summary, files, attachments, metadata, etc.)
}
```

### 1.2 Root Sentinel

Root-level messages (no parent) set `parentMessageId` to a constant sentinel UUID:

```
NO_PARENT = "00000000-0000-0000-0000-000000000000"
```

This avoids null checks and makes querying for roots simple.

### 1.3 What Creates a Branch

A branch is created whenever **two or more messages share the same `parentMessageId`**. These messages are called **siblings**. Branching occurs when:

- A user **edits and resubmits** a previous message — the new user message is saved with the same `parentMessageId` as the original.
- A user **regenerates** an assistant response — the new assistant response gets the same `parentMessageId` as the response being regenerated.
- An agent produces **parallel responses** that are split into separate messages.

No special flag marks a message as "branched." The tree structure emerges naturally from the parent pointers.

---

## 2. Building the Tree from Flat Messages

### 2.1 Fetching

All messages for a conversation are fetched in a single query:

```
db.messages.find({ conversationId, user }).sort({ createdAt: 1 })
```

This returns a **flat, chronologically-ordered array** of every message in the conversation — all branches included.

### 2.2 buildTree() — Constructing the Tree

**Location:** `packages/data-provider/src/messages.ts`

This pure function converts the flat array into a tree of `ParentMessage` nodes. Each node gains `children: ParentMessage[]`, `depth: number`, and `siblingIndex: number`.

Algorithm (single pass over the sorted array):

```
Input:  messages[] sorted by createdAt ascending
Output: rootMessages[] (forest of tree roots)

messageMap   = {}          // messageId -> ParentMessage
childrenCount = {}         // parentId  -> count of children seen so far

for each message in messages:
    parentId = message.parentMessageId ?? ""
    childrenCount[parentId] += 1

    node = {
        ...message,
        children: [],
        depth: 0,
        siblingIndex: childrenCount[parentId] - 1
    }

    messageMap[message.messageId] = node

    if messageMap[parentId] exists:
        messageMap[parentId].children.push(node)
        node.depth = messageMap[parentId].depth + 1
    else:
        rootMessages.push(node)

return rootMessages
```

Key properties:
- **Single-pass O(n)** — messages must be pre-sorted by `createdAt` ascending so that parents are always processed before children.
- **siblingIndex** is assigned in insertion order (chronological), so the first-created sibling is index 0, the most recent is last.
- The returned `rootMessages` array typically has one element (the first user message), but can have multiple roots if messages were imported or split.

---

## 3. Rendering the Active Branch Path

The tree is a full representation of every branch. The UI must select **one path** through the tree to display, while allowing the user to switch between siblings at any level.

### 3.1 Sibling Index State (Frontend)

The currently-selected sibling at each tree position is stored in a **Recoil atom family**:

```typescript
const messagesSiblingIdxFamily = atomFamily<number, string | null | undefined>({
  key: 'messagesSiblingIdx',
  default: 0,   // 0 = show the latest/newest sibling
});
```

The key is the **parent's messageId** (or the conversationId for root-level messages). The value is a reversed index — `0` means the **last** (most recent) child, `1` means second-to-last, etc.

### 3.2 Recursive Rendering

The rendering is a recursive loop of two components:

```
MessagesView
  └─ MultiMessage(messagesTree=rootMessages, messageId=conversationId)
       ├─ selects one sibling from messagesTree using siblingIdx
       └─ renders Message(message=selectedSibling)
            └─ MultiMessage(messagesTree=selectedSibling.children, messageId=selectedSibling.messageId)
                 ├─ selects one sibling...
                 └─ renders Message(...)
                      └─ ...recursion continues until a leaf node
```

**MultiMessage** is the branching decision point:

```
function MultiMessage({ messageId, messagesTree }):
    siblingIdx = recoilState(messagesSiblingIdxFamily(messageId))  // default 0

    if messagesTree has only 1 child:
        selectedMessage = messagesTree[0]
    else:
        // Reverse indexing: idx 0 = last element (newest)
        selectedMessage = messagesTree[messagesTree.length - siblingIdx - 1]

    render <Message message={selectedMessage}
                    siblingIdx={messagesTree.length - siblingIdx - 1}
                    siblingCount={messagesTree.length}
                    setSiblingIdx={reversedSetter} />
```

**Message** renders its own content, then recurses:

```
function Message({ message }):
    render <MessageContent ... />
    render <MultiMessage messagesTree={message.children}
                         messageId={message.messageId} />
```

### 3.3 Sibling Navigation UI (SiblingSwitch)

When `siblingCount > 1`, the message renders a `SiblingSwitch` nav component:

```
[ < ]  2 / 3  [ > ]
```

- Displays `siblingIdx + 1` / `siblingCount` (1-indexed for display).
- Previous/Next buttons call `setSiblingIdx(idx - 1)` / `setSiblingIdx(idx + 1)`.
- Buttons are disabled at bounds (idx 0 and idx siblingCount-1).

### 3.4 Auto-reset on New Messages

When `messagesTree.length` changes (new message submitted, regeneration completes), `siblingIdx` resets to `0` — which shows the newest sibling. This ensures users always see the latest response after submitting.

---

## 4. Message Editing

### 4.1 Edit-in-Place (Save Only)

**Endpoint:** `PUT /api/messages/:conversationId/:messageId`
**Payload:** `{ text: string }`

Updates the message's `text` field in the database. No new messages are created. The tree structure is unchanged. The frontend updates its React Query cache optimistically.

### 4.2 Edit and Resubmit (Save & Submit)

This is the operation that **creates a branch**:

1. User edits a previous user message and presses Ctrl+Enter.
2. The frontend creates a **new message** with:
   - A new `messageId` (UUID)
   - The **same `parentMessageId`** as the original message being edited
   - The edited `text`
   - `isCreatedByUser: true`
3. This new message is submitted to the AI endpoint for completion.
4. The AI response is saved as a child of the new user message.

After this, the original message and the new message are **siblings** — they share the same parent. The `siblingCount` at that tree level increases by 1, and the sibling navigator appears.

### 4.3 Regeneration

Similar to edit-and-resubmit but without changing the user message:

1. User clicks "regenerate" on an assistant response.
2. A new assistant message is created with the **same `parentMessageId`** as the existing response.
3. The new response becomes a sibling of the original.

---

## 5. Resolving a Linear Path from the Tree (Backend)

When the backend needs to send conversation history to the AI model, it must resolve a single linear chain of messages. This is done by walking **backwards** from a target message to the root.

### 5.1 getMessagesForConversation (Ancestor Chain Walk)

**Location:** `api/app/clients/BaseClient.js`

```
Input:  messages[] (flat array), parentMessageId (the leaf to walk back from)
Output: orderedMessages[] (root → ... → leaf, linear)

orderedMessages = []
currentId = parentMessageId
visited = Set()

while currentId is not null:
    if currentId in visited: break       // cycle guard
    visited.add(currentId)

    message = messages.find(m => m.messageId == currentId)
    if not message: break

    orderedMessages.push(message)
    currentId = message.parentMessageId == NO_PARENT ? null : message.parentMessageId

orderedMessages.reverse()
return orderedMessages
```

This walks from the **latest message being replied to** (the leaf) up to the root, collecting the direct ancestor chain. The result is the linear history that gets sent to the model.

Key insight: **The model never sees sibling branches.** It only sees the single path of messages leading to the current point in the conversation.

---

## 6. Forking Conversations

Forking creates a **new conversation** from a subset of an existing conversation's message tree. This is distinct from in-conversation branching.

### 6.1 Fork Modes

```
enum ForkOptions {
  DIRECT_PATH     = "directPath"       // Only the ancestor chain to the target
  INCLUDE_BRANCHES = "includeBranches"  // Ancestor chain + siblings of each ancestor
  TARGET_LEVEL    = "targetLevel"       // All messages from root down to target's depth
}
```

### 6.2 DIRECT_PATH

Uses `getMessagesForConversation()` (Section 5.1) — the same ancestor-chain walk used for AI context. Collects only messages on the direct path from root to target.

### 6.3 INCLUDE_BRANCHES

**Algorithm: getAllMessagesUpToParent()**

1. Walk from target to root, collecting the set of messageIds on the direct path (`pathToRoot`).
2. Include any message whose `parentMessageId` is in `pathToRoot` (siblings of ancestors).
3. Exclude children of the target message itself.

This gives the direct path plus all alternative branches at each level up to (and including) the target.

### 6.4 TARGET_LEVEL (Default)

**Algorithm: getMessagesUpToTargetLevel()**

Breadth-first traversal from root messages, level by level, collecting **every** message at every level. Stops once the level containing the target message is reached. This is the widest fork mode — it includes all branches and siblings at every level from root through the target's depth.

```
parentToChildrenMap = build from messages[]
currentLevel = rootMessages
results = Set(currentLevel)

while target not found in currentLevel:
    nextLevel = []
    for each node in currentLevel:
        for each child of node:
            nextLevel.push(child)
            results.add(child)
            if child.messageId == targetMessageId:
                targetFound = true
    currentLevel = nextLevel

return Array.from(results)
```

### 6.5 splitAtTarget Option

When `splitAtTarget = true`, the fork discards all ancestors and starts the new conversation at the target's tree depth:

1. Assign a `level` number to every message via BFS from root.
2. Discard messages at levels below the target level.
3. Remap messages AT the target level to have `parentMessageId = NO_PARENT` (they become roots).
4. Messages below the target level keep their original parent pointers.

This is used for "regenerate from here" scenarios where the user wants a fresh conversation starting at a specific point.

### 6.6 Cloning Process

After selecting which messages to include:

1. **Generate new IDs** — each message gets a new UUID `messageId`.
2. **Remap parent pointers** — `parentMessageId` is updated to point to the new ID of the parent (using an `oldId -> newId` map). Root messages point to `NO_PARENT`.
3. **Fix timestamps** — ensure each child's `createdAt` is strictly after its parent's (add 1ms if not). This preserves correct chronological ordering for `buildTree()`.
4. **Create new conversation** — copy metadata (title, endpoint, model, etc.) from the original, assign a new `conversationId`.
5. **Batch save** — insert all cloned messages and the new conversation in a batch.

---

## 7. Visual Summary of the Tree

```
Conversation: conv_abc

                    [User msg A]          ← root (parentMessageId = NO_PARENT)
                    /           \
          [AI response B]    [AI response B']    ← siblings (both parentMessageId = A)
               |                    |
          [User msg C]         [User msg C']
           /        \               |
    [AI resp D]  [AI resp D']  [AI resp D'']
        |
   [User msg E]
        |
   [AI resp F]

Database stores all 10 messages as flat rows.
UI shows one path (e.g., A → B → C → D → E → F).
Sibling navigators appear at B/B' and D/D' levels.
AI context for F = [A, B, C, D, E, F] (linear ancestor chain).
```

---

## 8. Key Identifiers and Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `NO_PARENT` | `"00000000-0000-0000-0000-000000000000"` | Sentinel for root messages |
| `NEW_CONVO` | `"new"` | URL param for new conversation (no messages yet) |
| `messageId` | UUID v4 | Unique per message |
| `conversationId` | UUID v4 or generated | Groups messages into one conversation |

---

## 9. Key Implementation Files (Reference)

| Concern | File |
|---------|------|
| Message DB schema | `packages/data-schemas/src/schema/message.ts` |
| Tree builder (shared) | `packages/data-provider/src/messages.ts` — `buildTree()` |
| Ancestor chain resolver | `api/app/clients/BaseClient.js` — `getMessagesForConversation()` |
| Fork logic | `api/server/utils/import/fork.js` |
| Sibling state atom | `client/src/store/families.ts` — `messagesSiblingIdxFamily` |
| Recursive renderer | `client/src/components/Chat/Messages/MultiMessage.tsx` |
| Single message + recurse | `client/src/components/Chat/Messages/Message.tsx` |
| Sibling navigation UI | `client/src/components/Chat/Messages/SiblingSwitch.tsx` |
| Top-level tree consumer | `client/src/components/Chat/ChatView.tsx` |
| Message list container | `client/src/components/Chat/Messages/MessagesView.tsx` |
| Edit UI | `client/src/components/Chat/Messages/Content/EditMessage.tsx` |
| Fork mutation (frontend) | `client/src/data-provider/mutations.ts` |
| ForkOptions enum | `packages/data-provider/src/config.ts` |
