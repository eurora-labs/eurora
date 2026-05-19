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

import { appendReasoningContent, readChunkReasoningDelta } from '$lib/models/messages/kwargs.js';
import type {
	ContentBlock,
	ReasoningContentBlock,
	TextContentBlock,
} from '$lib/models/content-blocks/index.js';
import type { AiMessageChunk, AiNode } from '$lib/models/messages/index.js';

export class AiStreamSink {
	private hasContent = false;
	private pendingWhitespace = '';

	constructor(public readonly placeholder: AiNode) {}

	/** The placeholder's id, never null because factories always assign one. */
	get id(): string {
		const id = this.placeholder.message.id;
		if (typeof id !== 'string' || id.length === 0) {
			throw new Error('AiStreamSink placeholder is missing an id');
		}
		return id;
	}

	/** True iff no content blocks have been appended yet. Callers use
	 *  this to decide whether to drop an empty placeholder on stream
	 *  error. */
	get isEmpty(): boolean {
		return this.placeholder.message.content.length === 0;
	}

	/** Merge one streaming chunk into the placeholder. Safe to call with
	 *  empty-content chunks (heartbeats from some providers). */
	appendChunk(chunk: AiMessageChunk): void {
		const reasoningDelta = readChunkReasoningDelta(chunk);
		if (reasoningDelta.length > 0) {
			appendReasoningContent(this.placeholder.message, reasoningDelta);
		}
		for (const block of chunk.content) {
			this.appendBlock(block);
		}
	}

	private appendBlock(block: ContentBlock): void {
		if (block.type === 'text') {
			this.appendText(block);
		} else if (block.type === 'reasoning') {
			this.appendReasoning(block);
		} else {
			this.placeholder.message.content.push(block);
		}
	}

	private appendText(block: { type: 'text' } & TextContentBlock): void {
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
		const existing = this.placeholder.message.content.find((b) => b.type === 'text');
		if (existing && existing.type === 'text') {
			existing.text += text;
		} else {
			this.placeholder.message.content.push({ ...block, text });
		}
	}

	private appendReasoning(block: { type: 'reasoning' } & ReasoningContentBlock): void {
		const existing = this.placeholder.message.content.find((b) => b.type === 'reasoning');
		if (existing && existing.type === 'reasoning') {
			existing.reasoning = (existing.reasoning ?? '') + (block.reasoning ?? '');
		} else {
			this.placeholder.message.content.push({ ...block });
		}
	}
}
