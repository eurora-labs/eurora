import {
	isLocalMessageId,
	isLocalThreadId,
	isPlaceholderId,
	isServerMessageId,
	newLocalMessageId,
	newLocalThreadId,
	newPlaceholderId,
} from '$lib/models/messages/ids.js';
import { describe, expect, it } from 'vitest';

const UUID_RE = /^[0-9a-f-]{36}$/i;

describe('client-issued ids', () => {
	it('mints placeholder ids with the placeholder: prefix and a UUID body', () => {
		const id = newPlaceholderId();
		expect(id.startsWith('placeholder:')).toBe(true);
		expect(id.slice('placeholder:'.length)).toMatch(UUID_RE);
	});

	it('mints local message ids with the local: prefix and a UUID body', () => {
		const id = newLocalMessageId();
		expect(id.startsWith('local:')).toBe(true);
		expect(id.slice('local:'.length)).toMatch(UUID_RE);
	});

	it('mints local thread ids with the local-thread: prefix and a UUID body', () => {
		const id = newLocalThreadId();
		expect(id.startsWith('local-thread:')).toBe(true);
		expect(id.slice('local-thread:'.length)).toMatch(UUID_RE);
	});

	it('returns distinct ids across calls', () => {
		const ids = new Set([
			newPlaceholderId(),
			newPlaceholderId(),
			newLocalMessageId(),
			newLocalThreadId(),
		]);
		expect(ids.size).toBe(4);
	});
});

describe('id guards', () => {
	it('isPlaceholderId narrows minted placeholder ids', () => {
		expect(isPlaceholderId(newPlaceholderId())).toBe(true);
	});

	it('isPlaceholderId rejects local and server ids', () => {
		expect(isPlaceholderId(newLocalMessageId())).toBe(false);
		expect(isPlaceholderId('00000000-0000-0000-0000-000000000000')).toBe(false);
		expect(isPlaceholderId('placeholderwithoutcolon')).toBe(false);
	});

	it('isLocalMessageId narrows minted local message ids', () => {
		expect(isLocalMessageId(newLocalMessageId())).toBe(true);
	});

	it('isLocalMessageId rejects placeholder and server ids', () => {
		expect(isLocalMessageId(newPlaceholderId())).toBe(false);
		expect(isLocalMessageId('00000000-0000-0000-0000-000000000000')).toBe(false);
	});

	it('isLocalThreadId narrows minted local thread ids', () => {
		expect(isLocalThreadId(newLocalThreadId())).toBe(true);
	});

	it('isLocalThreadId rejects placeholder, local-message, and server ids', () => {
		expect(isLocalThreadId(newPlaceholderId())).toBe(false);
		expect(isLocalThreadId(newLocalMessageId())).toBe(false);
		expect(isLocalThreadId('00000000-0000-0000-0000-000000000000')).toBe(false);
	});

	it('isServerMessageId classifies non-prefixed strings as server ids', () => {
		expect(isServerMessageId('00000000-0000-0000-0000-000000000000')).toBe(true);
		expect(isServerMessageId(newPlaceholderId())).toBe(false);
		expect(isServerMessageId(newLocalMessageId())).toBe(false);
	});

	it('isServerMessageId classifies the empty string as a server id', () => {
		// The empty string predates any prefix and falls through to the
		// "not placeholder, not local" bucket. The factories never emit it,
		// but defensive callers might pass it on a missing field.
		expect(isServerMessageId('')).toBe(true);
	});
});
