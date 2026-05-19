import {
	appendReasoningContent,
	readAssetChips,
	readChunkReasoningDelta,
	readReasoningContent,
	writeAssetChips,
} from '$lib/models/messages/kwargs.js';
import { describe, expect, it } from 'vitest';
import type { AiMessage, HumanMessage } from '$lib/models/messages/nodes.js';

function humanMessage(overrides: Partial<HumanMessage> = {}): HumanMessage {
	return {
		type: 'human',
		id: 'test',
		name: null,
		content: [],
		additional_kwargs: {},
		response_metadata: {},
		...overrides,
	};
}

function aiMessage(overrides: Partial<AiMessage> = {}): AiMessage {
	return {
		type: 'ai',
		id: 'test',
		name: null,
		content: [],
		tool_calls: [],
		invalid_tool_calls: [],
		usage_metadata: null,
		additional_kwargs: {},
		response_metadata: {},
		...overrides,
	};
}

describe('readAssetChips', () => {
	it('returns the persisted chips on a human message', () => {
		const m = humanMessage({
			additional_kwargs: {
				asset_chips: [
					{ id: 'a', name: 'A', icon: 'i', domain: 'd' },
					{ id: 'b', name: 'B', icon: null, domain: null },
				],
			},
		});
		expect(readAssetChips(m)).toEqual([
			{ id: 'a', name: 'A', icon: 'i', domain: 'd' },
			{ id: 'b', name: 'B', icon: null, domain: null },
		]);
	});

	it('drops entries missing id or name', () => {
		const m = humanMessage({
			additional_kwargs: {
				asset_chips: [
					{ id: 'a', name: 'A' },
					{ id: 'b' }, // missing name
					{ name: 'C' }, // missing id
					'not an object',
				],
			},
		});
		expect(readAssetChips(m)).toEqual([{ id: 'a', name: 'A', icon: null, domain: null }]);
	});

	it('coerces missing icon/domain to null', () => {
		const m = humanMessage({
			additional_kwargs: { asset_chips: [{ id: 'a', name: 'A' }] },
		});
		expect(readAssetChips(m)).toEqual([{ id: 'a', name: 'A', icon: null, domain: null }]);
	});

	it('returns [] for non-human messages, even if the wire data looks shaped', () => {
		const m = aiMessage({
			additional_kwargs: {
				asset_chips: [{ id: 'a', name: 'A', icon: null, domain: null }],
			},
		});
		expect(readAssetChips(m)).toEqual([]);
	});

	it('returns [] for missing kwargs', () => {
		expect(readAssetChips(humanMessage())).toEqual([]);
	});

	it('returns [] when asset_chips is not an array', () => {
		const m = humanMessage({ additional_kwargs: { asset_chips: 'oops' } });
		expect(readAssetChips(m)).toEqual([]);
	});

	it('returns [] for null/undefined input', () => {
		expect(readAssetChips(null)).toEqual([]);
		expect(readAssetChips(undefined)).toEqual([]);
	});
});

describe('writeAssetChips', () => {
	it('persists chips into the kwargs map', () => {
		const m = humanMessage();
		writeAssetChips(m, [{ id: 'a', name: 'A', icon: null, domain: null }]);
		expect(m.additional_kwargs?.asset_chips).toEqual([
			{ id: 'a', name: 'A', icon: null, domain: null },
		]);
	});

	it('drops the key entirely when given an empty list', () => {
		const m = humanMessage({
			additional_kwargs: { asset_chips: [{ id: 'a', name: 'A' }] },
		});
		writeAssetChips(m, []);
		expect(m.additional_kwargs).toEqual({});
	});

	it('preserves other kwargs when writing', () => {
		const m = humanMessage({
			additional_kwargs: { other: 'value' },
		});
		writeAssetChips(m, [{ id: 'a', name: 'A', icon: null, domain: null }]);
		expect(m.additional_kwargs).toEqual({
			other: 'value',
			asset_chips: [{ id: 'a', name: 'A', icon: null, domain: null }],
		});
	});
});

describe('readReasoningContent', () => {
	it('reads reasoning from content blocks when present', () => {
		const m = aiMessage({
			content: [
				{ type: 'reasoning', reasoning: 'first ', id: null, index: null, extras: null },
				{ type: 'reasoning', reasoning: 'second', id: null, index: null, extras: null },
			],
		});
		expect(readReasoningContent(m)).toBe('first second');
	});

	it('falls back to the additional_kwargs side-channel when no content blocks', () => {
		const m = aiMessage({
			additional_kwargs: { reasoning_content: 'inside kwargs' },
		});
		expect(readReasoningContent(m)).toBe('inside kwargs');
	});

	it('prefers content blocks over the kwarg side-channel when both are present', () => {
		const m = aiMessage({
			content: [
				{
					type: 'reasoning',
					reasoning: 'from blocks',
					id: null,
					index: null,
					extras: null,
				},
			],
			additional_kwargs: { reasoning_content: 'from kwargs' },
		});
		expect(readReasoningContent(m)).toBe('from blocks');
	});

	it('returns "" for messages without reasoning', () => {
		expect(readReasoningContent(aiMessage())).toBe('');
		expect(readReasoningContent(humanMessage())).toBe('');
		expect(readReasoningContent(null)).toBe('');
		expect(readReasoningContent(undefined)).toBe('');
	});
});

describe('appendReasoningContent', () => {
	it('seeds the kwarg on first append', () => {
		const m = aiMessage();
		appendReasoningContent(m, 'hello ');
		expect(m.additional_kwargs?.reasoning_content).toBe('hello ');
	});

	it('concatenates onto an existing kwarg', () => {
		const m = aiMessage({ additional_kwargs: { reasoning_content: 'one ' } });
		appendReasoningContent(m, 'two');
		expect(m.additional_kwargs?.reasoning_content).toBe('one two');
	});

	it('is a no-op for an empty delta', () => {
		const m = aiMessage();
		appendReasoningContent(m, '');
		expect(m.additional_kwargs).toEqual({});
	});
});

describe('readChunkReasoningDelta', () => {
	it('reads the reasoning_content string off a chunk', () => {
		expect(readChunkReasoningDelta({ additional_kwargs: { reasoning_content: 'delta' } })).toBe(
			'delta',
		);
	});

	it('returns "" when the kwarg is missing', () => {
		expect(readChunkReasoningDelta({})).toBe('');
		expect(readChunkReasoningDelta({ additional_kwargs: {} })).toBe('');
	});

	it('returns "" when the value is non-string', () => {
		expect(readChunkReasoningDelta({ additional_kwargs: { reasoning_content: 42 } })).toBe('');
	});
});
