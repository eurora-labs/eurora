import { createAiPlaceholderNode, type AiPlaceholderNode } from '$lib/models/messages/factory.js';
import { AiStreamSink } from '$lib/models/messages/stream-sink.js';
import { describe, expect, it, vi } from 'vitest';
import type { AiMessageChunk } from '$lib/models/messages/index.js';

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

/// Build a sink over `node` and return both — assertions read directly
/// from `node`, mirroring the production pattern where the resolver
/// returns the live (reactive) view of the placeholder.
function sinkOver(node: AiPlaceholderNode = createAiPlaceholderNode(null)): {
	sink: AiStreamSink;
	node: AiPlaceholderNode;
} {
	return {
		sink: new AiStreamSink(node.message.id, () => node),
		node,
	};
}

function textBlock(text: string): AiMessageChunk['content'][0] {
	return {
		type: 'text' as const,
		text,
		id: null,
		annotations: null,
		index: null,
		extras: null,
	};
}

function reasoningBlock(reasoning: string | null): AiMessageChunk['content'][0] {
	return {
		type: 'reasoning' as const,
		reasoning,
		id: null,
		index: null,
		extras: null,
	};
}

describe('AiStreamSink.id', () => {
	it('exposes the id supplied to the constructor', () => {
		const placeholder = createAiPlaceholderNode(null);
		const sink = new AiStreamSink(placeholder.message.id, () => placeholder);
		expect(sink.id).toBe(placeholder.message.id);
	});
});

describe('AiStreamSink.isEmpty', () => {
	it('is true for a fresh placeholder', () => {
		const { sink } = sinkOver();
		expect(sink.isEmpty).toBe(true);
	});

	it('flips to false after the first content append', () => {
		const { sink } = sinkOver();
		sink.appendChunk(chunk([textBlock('hi')]));
		expect(sink.isEmpty).toBe(false);
	});

	it('is true when the resolver returns null (placeholder removed)', () => {
		const sink = new AiStreamSink('placeholder:gone', () => null);
		expect(sink.isEmpty).toBe(true);
	});
});

describe('AiStreamSink text streaming', () => {
	it('concatenates text across chunks into a single block', () => {
		const { sink, node } = sinkOver();
		sink.appendChunk(chunk([textBlock('hello ')]));
		sink.appendChunk(chunk([textBlock('world')]));
		expect(node.message.content).toHaveLength(1);
		expect(node.message.content[0]).toMatchObject({ type: 'text', text: 'hello world' });
	});

	it('buffers leading whitespace until real content arrives, then flushes', () => {
		const { sink, node } = sinkOver();
		// Three whitespace-only chunks must NOT create a content block.
		sink.appendChunk(chunk([textBlock(' ')]));
		sink.appendChunk(chunk([textBlock('\n')]));
		sink.appendChunk(chunk([textBlock('\t')]));
		expect(sink.isEmpty).toBe(true);

		// The first non-whitespace token flushes the buffered prefix.
		sink.appendChunk(chunk([textBlock('hi')]));
		expect(node.message.content).toHaveLength(1);
		expect(node.message.content[0]).toMatchObject({ type: 'text', text: ' \n\thi' });
	});

	it('does not gate whitespace once content has started flowing', () => {
		const { sink, node } = sinkOver();
		sink.appendChunk(chunk([textBlock('first')]));
		sink.appendChunk(chunk([textBlock('   ')]));
		sink.appendChunk(chunk([textBlock('second')]));
		expect(node.message.content[0]).toMatchObject({
			type: 'text',
			text: 'first   second',
		});
	});
});

describe('AiStreamSink reasoning', () => {
	it('concatenates reasoning content blocks into a single block', () => {
		const { sink, node } = sinkOver();
		sink.appendChunk(chunk([reasoningBlock('thought ')]));
		sink.appendChunk(chunk([reasoningBlock('process')]));
		const blocks = node.message.content.filter((b) => b.type === 'reasoning');
		expect(blocks).toHaveLength(1);
		expect(blocks[0]).toMatchObject({ type: 'reasoning', reasoning: 'thought process' });
	});

	it('coalesces null/undefined reasoning into empty strings', () => {
		const { sink, node } = sinkOver();
		sink.appendChunk(chunk([reasoningBlock(null)]));
		sink.appendChunk(chunk([reasoningBlock('real')]));
		const blocks = node.message.content.filter((b) => b.type === 'reasoning');
		expect(blocks).toHaveLength(1);
		expect(blocks[0]).toMatchObject({ type: 'reasoning', reasoning: 'real' });
	});

	it('appends reasoning_content kwarg deltas to the placeholder kwargs', () => {
		const { sink, node } = sinkOver();
		sink.appendChunk(chunk([], { reasoning_content: 'first ' }));
		sink.appendChunk(chunk([], { reasoning_content: 'second' }));
		expect(node.message.additional_kwargs?.reasoning_content).toBe('first second');
	});

	it('ignores non-string reasoning_content kwargs', () => {
		const { sink, node } = sinkOver();
		sink.appendChunk(chunk([], { reasoning_content: 42 }));
		expect(node.message.additional_kwargs?.reasoning_content).toBeUndefined();
	});
});

describe('AiStreamSink passthrough blocks', () => {
	it('appends non-text / non-reasoning blocks verbatim', () => {
		const { sink, node } = sinkOver();
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
		expect(node.message.content).toHaveLength(1);
		expect(node.message.content[0]).toMatchObject({ type: 'tool_call', id: 'call-1' });
	});

	it('tolerates heartbeat chunks with empty content', () => {
		const { sink, node } = sinkOver();
		sink.appendChunk(chunk());
		expect(sink.isEmpty).toBe(true);
		expect(node.message.content).toEqual([]);
	});
});

describe('AiStreamSink resolver semantics', () => {
	it('drops chunks silently when the resolver returns null', () => {
		// Stand-in for a cancelled / thread-switched / error-rolled-back
		// placeholder. The sink must not throw and must not buffer state
		// that leaks into the next round.
		const sink = new AiStreamSink('placeholder:gone', () => null);
		expect(() => sink.appendChunk(chunk([textBlock('hi')]))).not.toThrow();
		expect(sink.isEmpty).toBe(true);
	});

	it('calls the resolver on every appendChunk, not just construction', () => {
		// Pins the live-lookup contract: a sink that cached the node on
		// construction would call the resolver once. The chat-service
		// fix relies on the per-call lookup to read through Svelte's
		// reactivity proxy.
		const node = createAiPlaceholderNode(null);
		const resolve = vi.fn(() => node);
		const sink = new AiStreamSink(node.message.id, resolve);

		sink.appendChunk(chunk([textBlock('one')]));
		sink.appendChunk(chunk([textBlock(' two')]));
		sink.appendChunk(chunk([textBlock(' three')]));

		expect(resolve).toHaveBeenCalledTimes(3);
	});

	it('routes every mutation through whatever object the resolver returns', () => {
		// Regression guard for the Svelte-reactivity bug: a Proxy that
		// records every `set` stands in for `$state`'s tracking proxy.
		// Mutations triggered by `appendChunk` must surface as writes
		// observed by the proxy — otherwise reactive consumers never
		// see the streaming update and the UI stalls until the final
		// swap.
		const underlying = createAiPlaceholderNode(null);
		const writes: string[] = [];
		const proxied = new Proxy(underlying, {
			get(target, key, receiver) {
				const value = Reflect.get(target, key, receiver);
				if (typeof value === 'object' && value !== null) {
					return new Proxy(value, this);
				}
				return value;
			},
			set(target, key, value, receiver) {
				writes.push(String(key));
				return Reflect.set(target, key, value, receiver);
			},
		}) as AiPlaceholderNode;

		const sink = new AiStreamSink(underlying.message.id, () => proxied);
		sink.appendChunk(chunk([textBlock('hello')]));

		expect(writes.length).toBeGreaterThan(0);
		// The underlying object should also reflect the change (the
		// proxy forwards writes to the target).
		expect(underlying.message.content[0]).toMatchObject({ type: 'text', text: 'hello' });
	});
});
