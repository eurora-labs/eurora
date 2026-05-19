import { createAiPlaceholderNode } from '$lib/models/messages/factory.js';
import { AiStreamSink } from '$lib/models/messages/stream-sink.js';
import { describe, expect, it } from 'vitest';
import type { AiMessageChunk, AiNode } from '$lib/models/messages/index.js';

function chunk(
	content: AiMessageChunk['content'] = [],
	additional_kwargs?: { [key: string]: unknown },
): AiMessageChunk {
	return {
		content,
		tool_call_chunks: [],
		additional_kwargs,
	};
}

describe('AiStreamSink.id', () => {
	it('returns the placeholder id', () => {
		const placeholder = createAiPlaceholderNode(null);
		const sink = new AiStreamSink(placeholder);
		expect(sink.id).toBe(placeholder.message.id);
	});

	it('throws when the placeholder has no id (defensive)', () => {
		// The factories always set an id, so a node with `id: undefined`
		// only happens if a caller wraps the sink around a server-shaped
		// `AiNode` whose wire data dropped it. Construct that case
		// directly rather than mutating a placeholder.
		const node: AiNode = {
			parent_id: null,
			message: {
				type: 'ai',
				content: [],
				tool_calls: [],
				invalid_tool_calls: [],
				usage_metadata: null,
				additional_kwargs: {},
				response_metadata: {},
			},
			children: [],
			sibling_index: 0,
			depth: 0,
		};
		const sink = new AiStreamSink(node);
		expect(() => sink.id).toThrow(/missing an id/);
	});
});

describe('AiStreamSink.isEmpty', () => {
	it('is true for a fresh placeholder', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		expect(sink.isEmpty).toBe(true);
	});

	it('flips to false after the first content append', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		sink.appendChunk(
			chunk([
				{
					type: 'text',
					text: 'hi',
					id: null,
					annotations: null,
					index: null,
					extras: null,
				},
			]),
		);
		expect(sink.isEmpty).toBe(false);
	});
});

describe('AiStreamSink text streaming', () => {
	it('concatenates text across chunks into a single block', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		sink.appendChunk(
			chunk([
				{
					type: 'text',
					text: 'hello ',
					id: null,
					annotations: null,
					index: null,
					extras: null,
				},
			]),
		);
		sink.appendChunk(
			chunk([
				{
					type: 'text',
					text: 'world',
					id: null,
					annotations: null,
					index: null,
					extras: null,
				},
			]),
		);
		expect(sink.placeholder.message.content).toHaveLength(1);
		expect(sink.placeholder.message.content[0]).toMatchObject({
			type: 'text',
			text: 'hello world',
		});
	});

	it('buffers leading whitespace until real content arrives, then flushes', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		// Three whitespace-only chunks should NOT create a content block.
		sink.appendChunk(
			chunk([
				{ type: 'text', text: ' ', id: null, annotations: null, index: null, extras: null },
			]),
		);
		sink.appendChunk(
			chunk([
				{
					type: 'text',
					text: '\n',
					id: null,
					annotations: null,
					index: null,
					extras: null,
				},
			]),
		);
		sink.appendChunk(
			chunk([
				{
					type: 'text',
					text: '\t',
					id: null,
					annotations: null,
					index: null,
					extras: null,
				},
			]),
		);
		expect(sink.isEmpty).toBe(true);

		// The first non-whitespace token flushes the buffered prefix.
		sink.appendChunk(
			chunk([
				{
					type: 'text',
					text: 'hi',
					id: null,
					annotations: null,
					index: null,
					extras: null,
				},
			]),
		);
		expect(sink.placeholder.message.content).toHaveLength(1);
		expect(sink.placeholder.message.content[0]).toMatchObject({
			type: 'text',
			text: ' \n\thi',
		});
	});

	it('does not gate whitespace once content has started flowing', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		sink.appendChunk(
			chunk([
				{
					type: 'text',
					text: 'first',
					id: null,
					annotations: null,
					index: null,
					extras: null,
				},
			]),
		);
		sink.appendChunk(
			chunk([
				{
					type: 'text',
					text: '   ',
					id: null,
					annotations: null,
					index: null,
					extras: null,
				},
			]),
		);
		sink.appendChunk(
			chunk([
				{
					type: 'text',
					text: 'second',
					id: null,
					annotations: null,
					index: null,
					extras: null,
				},
			]),
		);
		expect(sink.placeholder.message.content[0]).toMatchObject({
			type: 'text',
			text: 'first   second',
		});
	});
});

describe('AiStreamSink reasoning', () => {
	it('concatenates reasoning content blocks into a single block', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		sink.appendChunk(
			chunk([
				{ type: 'reasoning', reasoning: 'thought ', id: null, index: null, extras: null },
			]),
		);
		sink.appendChunk(
			chunk([
				{ type: 'reasoning', reasoning: 'process', id: null, index: null, extras: null },
			]),
		);
		const blocks = sink.placeholder.message.content.filter((b) => b.type === 'reasoning');
		expect(blocks).toHaveLength(1);
		expect(blocks[0]).toMatchObject({ type: 'reasoning', reasoning: 'thought process' });
	});

	it('coalesces null/undefined reasoning into empty strings', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		sink.appendChunk(
			chunk([{ type: 'reasoning', reasoning: null, id: null, index: null, extras: null }]),
		);
		sink.appendChunk(
			chunk([{ type: 'reasoning', reasoning: 'real', id: null, index: null, extras: null }]),
		);
		const blocks = sink.placeholder.message.content.filter((b) => b.type === 'reasoning');
		expect(blocks).toHaveLength(1);
		expect(blocks[0]).toMatchObject({ type: 'reasoning', reasoning: 'real' });
	});

	it('appends reasoning_content kwarg deltas to the placeholder kwargs', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		sink.appendChunk(chunk([], { reasoning_content: 'first ' }));
		sink.appendChunk(chunk([], { reasoning_content: 'second' }));
		expect(sink.placeholder.message.additional_kwargs?.reasoning_content).toBe('first second');
	});

	it('ignores non-string reasoning_content kwargs', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		sink.appendChunk(chunk([], { reasoning_content: 42 }));
		expect(sink.placeholder.message.additional_kwargs?.reasoning_content).toBeUndefined();
	});
});

describe('AiStreamSink passthrough blocks', () => {
	it('appends non-text / non-reasoning blocks verbatim', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		sink.appendChunk(
			chunk([
				{
					type: 'tool_call',
					id: 'call-1',
					name: 'foo',
					args: {},
					index: null,
					extras: null,
				},
			]),
		);
		expect(sink.placeholder.message.content).toHaveLength(1);
		expect(sink.placeholder.message.content[0]).toMatchObject({
			type: 'tool_call',
			id: 'call-1',
		});
	});

	it('tolerates heartbeat chunks with empty content', () => {
		const sink = new AiStreamSink(createAiPlaceholderNode(null));
		sink.appendChunk(chunk());
		expect(sink.isEmpty).toBe(true);
		expect(sink.placeholder.message.content).toEqual([]);
	});
});
