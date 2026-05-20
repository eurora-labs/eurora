// `AiStreamSink` owns the incremental mutation of a streaming AI
// placeholder. The class folds three concerns the chat handler used to
// scatter inline:
//
// 1. *Whitespace gating* — providers sometimes prefix their first chunks
//    with a stray space/newline. The sink buffers leading-whitespace
//    text until a non-whitespace chunk arrives, then flushes the
//    accumulated whitespace alongside the first real token. This keeps
//    the UI from showing a "phantom space" before the model speaks.
// 2. *Single-text-block convention* — agent-chain streams text as many
//    chunks but the rendered message holds a single text block. The
//    sink finds-or-creates that block on the placeholder and appends.
//    Reasoning blocks follow the same find-or-append convention.
// 3. *Reasoning kwarg accumulation* — DeepSeek/Ollama/XAI emit reasoning
//    in `additional_kwargs.reasoning_content` rather than as content
//    blocks. The sink delegates to `appendReasoningContent` so the
//    kwarg shape lives in one place.
//
// ## Live-lookup contract
//
// The sink does not hold a reference to the placeholder. It is
// constructed with a `resolve` closure that returns the *current*
// placeholder on every call. Production callers route the closure
// through a lookup against a Svelte `$state`-backed array — so each
// mutation passes through the reactive proxy and the UI re-renders
// incrementally. Holding a captured reference would silently bypass
// the proxy and defer all rendering until the array is reassigned
// (which is exactly the streaming regression this design prevents).
//
// The resolver may legitimately return `null` (cancellation, thread
// switch, error rollback); the sink treats that as "drop the chunk"
// — no throws, no partial mutations.

import { appendReasoningContent, readChunkReasoningDelta } from '$lib/models/messages/kwargs.js';
import type {
	ContentBlock,
	ReasoningContentBlock,
	TextContentBlock,
} from '$lib/models/content-blocks/index.js';
import type { AiPlaceholderNode } from '$lib/models/messages/factory.js';
import type { AiMessageChunk } from '$lib/models/messages/index.js';

/// Returns the live, reactive placeholder node — or `null` if the
/// placeholder has been removed from the thread (cancellation, thread
/// switch, error rollback). Called once per [`AiStreamSink`] mutation
/// so the read always goes through whatever reactivity proxy the
/// consumer's storage layer wraps the node in.
export type AiPlaceholderResolver = () => AiPlaceholderNode | null;

export class AiStreamSink {
	private hasContent = false;
	private pendingWhitespace = '';

	/// `id` is the stable placeholder identifier — consumers use it as
	/// the swap target when `final` arrives and as the filter key on
	/// error rollback.
	///
	/// `resolve` is called on every mutation. It MUST return the
	/// reactive view of the placeholder, not a captured reference:
	/// passing the raw object the placeholder was constructed from
	/// silently bypasses the consumer's reactivity proxy and the UI
	/// stops updating mid-stream.
	constructor(
		public readonly id: string,
		private readonly resolve: AiPlaceholderResolver,
	) {}

	/// True iff the placeholder has been removed *or* still has no
	/// content blocks. Callers use this on stream error to decide
	/// whether to drop the empty placeholder from the thread.
	get isEmpty(): boolean {
		const node = this.resolve();
		return node === null || node.message.content.length === 0;
	}

	/// Merge one streaming chunk into the placeholder. Safe to call
	/// with empty-content chunks (heartbeats from some providers) and
	/// safe to call after the placeholder has been removed — the
	/// resolver returns `null` and the chunk is dropped.
	appendChunk(chunk: AiMessageChunk): void {
		const node = this.resolve();
		if (node === null) return;

		const reasoningDelta = readChunkReasoningDelta(chunk);
		if (reasoningDelta.length > 0) {
			appendReasoningContent(node.message, reasoningDelta);
		}
		for (const block of chunk.content) {
			this.appendBlock(node, block);
		}
	}

	private appendBlock(node: AiPlaceholderNode, block: ContentBlock): void {
		if (block.type === 'text') {
			this.appendText(node, block);
		} else if (block.type === 'reasoning') {
			this.appendReasoning(node, block);
		} else {
			node.message.content.push(block);
		}
	}

	private appendText(node: AiPlaceholderNode, block: { type: 'text' } & TextContentBlock): void {
		let text = block.text;
		if (!this.hasContent) {
			if (text.trim().length === 0) {
				this.pendingWhitespace += text;
				return;
			}
			this.hasContent = true;
			text = this.pendingWhitespace + text;
			this.pendingWhitespace = '';
		}
		const existing = node.message.content.find((b) => b.type === 'text');
		if (existing && existing.type === 'text') {
			existing.text += text;
		} else {
			node.message.content.push({ ...block, text });
		}
	}

	private appendReasoning(
		node: AiPlaceholderNode,
		block: { type: 'reasoning' } & ReasoningContentBlock,
	): void {
		const existing = node.message.content.find((b) => b.type === 'reasoning');
		if (existing && existing.type === 'reasoning') {
			existing.reasoning = (existing.reasoning ?? '') + (block.reasoning ?? '');
		} else {
			node.message.content.push({ ...block });
		}
	}
}
