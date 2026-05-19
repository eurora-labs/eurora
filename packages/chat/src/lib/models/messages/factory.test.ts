import {
	createAiPlaceholderNode,
	createHumanPlaceholderNode,
	createLocalAiNode,
	createLocalHumanNode,
	createStubThread,
} from '$lib/models/messages/factory.js';
import { isLocalMessageId, isLocalThreadId, isPlaceholderId } from '$lib/models/messages/ids.js';
import { describe, expect, it } from 'vitest';

describe('createAiPlaceholderNode', () => {
	it('mints a placeholder id', () => {
		const node = createAiPlaceholderNode(null);
		expect(isPlaceholderId(node.message.id)).toBe(true);
	});

	it('stores the parent id verbatim', () => {
		const parent = '11111111-2222-3333-4444-555555555555';
		expect(createAiPlaceholderNode(parent).parent_id).toBe(parent);
		expect(createAiPlaceholderNode(null).parent_id).toBeNull();
	});

	it('seeds an empty content list when no text is provided', () => {
		expect(createAiPlaceholderNode(null).message.content).toEqual([]);
	});

	it('seeds a single text content block when text is provided', () => {
		const node = createAiPlaceholderNode(null, 'hello');
		expect(node.message.content).toEqual([
			{ type: 'text', id: null, text: 'hello', annotations: null, index: null, extras: null },
		]);
	});

	it('initializes the agentic fields to empty defaults', () => {
		const { message } = createAiPlaceholderNode(null);
		expect(message.tool_calls).toEqual([]);
		expect(message.invalid_tool_calls).toEqual([]);
		expect(message.usage_metadata).toBeNull();
		expect(message.additional_kwargs).toEqual({});
		expect(message.response_metadata).toEqual({});
	});
});

describe('createHumanPlaceholderNode', () => {
	it('mints a placeholder id', () => {
		const node = createHumanPlaceholderNode(null, 'hi');
		expect(isPlaceholderId(node.message.id)).toBe(true);
	});

	it('encodes the text into a single content block', () => {
		const node = createHumanPlaceholderNode(null, 'hi');
		expect(node.message.content).toHaveLength(1);
		expect(node.message.content[0]).toMatchObject({ type: 'text', text: 'hi' });
	});

	it('writes asset chips into additional_kwargs only when non-empty', () => {
		const empty = createHumanPlaceholderNode(null, 'hi');
		expect(empty.message.additional_kwargs).toEqual({});

		const withChips = createHumanPlaceholderNode(null, 'hi', [
			{ id: 'a', name: 'A', icon: null, domain: null },
		]);
		expect(withChips.message.additional_kwargs?.asset_chips).toEqual([
			{ id: 'a', name: 'A', icon: null, domain: null },
		]);
	});
});

describe('createLocalAiNode / createLocalHumanNode', () => {
	it('mints local: ids for AI nodes', () => {
		const node = createLocalAiNode(null, 'hi');
		expect(isLocalMessageId(node.message.id)).toBe(true);
		expect(isPlaceholderId(node.message.id)).toBe(false);
	});

	it('mints local: ids for human nodes', () => {
		const node = createLocalHumanNode(null, 'hi');
		expect(isLocalMessageId(node.message.id)).toBe(true);
	});
});

describe('createStubThread', () => {
	it('defaults to a local-thread: id when none is provided', () => {
		const t = createStubThread();
		expect(isLocalThreadId(t.id)).toBe(true);
	});

	it('respects an explicitly provided id', () => {
		const t = createStubThread('22222222-3333-4444-5555-666666666666');
		expect(t.id).toBe('22222222-3333-4444-5555-666666666666');
	});

	it('seeds the timestamps to the current time', () => {
		const before = Date.now();
		const t = createStubThread();
		const after = Date.now();
		const created = Date.parse(t.created_at);
		expect(created).toBeGreaterThanOrEqual(before);
		expect(created).toBeLessThanOrEqual(after);
		expect(t.created_at).toBe(t.updated_at);
	});
});
