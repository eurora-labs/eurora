// Client-side identity for messages and threads.
//
// Server-persisted records carry plain UUID strings. The client overlays
// two flavors of locally-issued identity on top:
//
// - **Placeholder** — a node we created locally and expect the server to
//   ratify; replaced with the persisted node when `final` arrives.
// - **Local** — a node that never syncs (the transient thread that ships
//   with a synthetic exchange to demo a feature).
//
// The prefixes `placeholder:` and `local:` use a colon, which is illegal
// in a UUID, so a server-issued ID can never collide with a client-issued
// one. Template-literal types let `isPlaceholderId` / `isLocalMessageId`
// narrow callers without runtime regex.

export type PlaceholderId = `placeholder:${string}`;
export type LocalMessageId = `local:${string}`;
export type LocalThreadId = `local-thread:${string}`;

/** Mint a new placeholder ID for a node we expect the server to ratify. */
export function newPlaceholderId(): PlaceholderId {
	return `placeholder:${crypto.randomUUID()}`;
}

/** Mint a new ID for a message that will never sync to the server. */
export function newLocalMessageId(): LocalMessageId {
	return `local:${crypto.randomUUID()}`;
}

/** Mint a new ID for a transient thread shown only in the local UI. */
export function newLocalThreadId(): LocalThreadId {
	return `local-thread:${crypto.randomUUID()}`;
}

export function isPlaceholderId(id: string): id is PlaceholderId {
	return id.startsWith('placeholder:');
}

export function isLocalMessageId(id: string): id is LocalMessageId {
	return id.startsWith('local:');
}

export function isLocalThreadId(id: string): id is LocalThreadId {
	return id.startsWith('local-thread:');
}

/** Server-issued IDs are everything that isn't placeholder- or local-tagged. */
export function isServerMessageId(id: string): boolean {
	return !isPlaceholderId(id) && !isLocalMessageId(id);
}
