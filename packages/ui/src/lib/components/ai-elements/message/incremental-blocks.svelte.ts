import { parseBlocks, type Extension } from 'svelte-streamdown';

/**
 * Memoized incremental wrapper around Streamdown's `parseBlocks`.
 *
 * As long as `content` grows by appending and the extension set is stable,
 * only the trailing block — the part that could still change — is re-lexed
 * on each call. Stable blocks survive unchanged so the matching child
 * `Streamdown` instances diff to a no-op.
 *
 * Returns the same string identities for stable blocks across calls — the
 * Svelte `#each` block keying and downstream `$derived` caches both rely on
 * that to elide work.
 *
 * Correctness: re-lexing `(prev_last_block.raw + new_suffix)` produces the
 * same trailing block list as `parseBlocks(full_content).slice(stable)` for
 * all standard Markdown. The only construct where appended content can
 * retroactively change parsing of an earlier line is the setext heading
 * (`title\n===` / `title\n---`) — and that's locally contained, always
 * within the previous-last-block, so re-including that block in the
 * re-lex covers it.
 */
export class IncrementalBlocks {
	#blocks: string[] = [];
	#content = '';
	#extensions: readonly Extension[] = [];

	derive(
		content: string,
		extensions: readonly Extension[] = EMPTY_EXTENSIONS,
	): readonly string[] {
		const extensionsChanged = !sameExtensions(extensions, this.#extensions);

		if (!extensionsChanged && content === this.#content) {
			return this.#blocks;
		}

		// Full reset on extension change, content shrink, or non-prefix
		// divergence (e.g. user-triggered regenerate). All cheap edge cases —
		// the next streaming response will resume incremental from here.
		if (
			extensionsChanged ||
			content.length < this.#content.length ||
			!content.startsWith(this.#stablePrefix())
		) {
			this.#extensions = extensions;
			this.#content = content;
			this.#blocks = parseBlocks(content, [...extensions]);
			return this.#blocks;
		}

		const stable = this.#blocks.slice(0, -1);
		const stableLen = this.#stablePrefixLength();
		const tailSlice = content.slice(stableLen);
		const tailBlocks = parseBlocks(tailSlice, [...extensions]);

		this.#content = content;
		this.#blocks = stable.length === 0 ? tailBlocks : [...stable, ...tailBlocks];
		return this.#blocks;
	}

	#stablePrefixLength(): number {
		if (this.#blocks.length <= 1) return 0;
		let len = 0;
		for (let i = 0; i < this.#blocks.length - 1; i++) len += this.#blocks[i].length;
		return len;
	}

	#stablePrefix(): string {
		const len = this.#stablePrefixLength();
		return len === 0 ? '' : this.#content.slice(0, len);
	}
}

const EMPTY_EXTENSIONS: readonly Extension[] = Object.freeze([]);

function sameExtensions(a: readonly Extension[], b: readonly Extension[]): boolean {
	if (a === b) return true;
	if (a.length !== b.length) return false;
	for (let i = 0; i < a.length; i++) {
		if (a[i] !== b[i]) return false;
	}
	return true;
}
