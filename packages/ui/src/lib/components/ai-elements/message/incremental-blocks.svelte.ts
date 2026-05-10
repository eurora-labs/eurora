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
 * Correctness: re-lexing `(prev_last_block + new_suffix)` produces the same
 * trailing block list as `parseBlocks(full_content).slice(stable)` for all
 * standard Markdown. We can't slice `content` by `sum(stable_lengths)`
 * because `parseBlocks` drops the inter-block separator (blank-line) tokens,
 * so summed block lengths fall short of the actual content position. The
 * only construct where appended content can retroactively change parsing of
 * an earlier line is the setext heading (`title\n===` / `title\n---`) —
 * locally contained within the previous-last block, so re-including that
 * block in the re-lex covers it.
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

		// Full re-parse on first call, extension change, content shrink, or
		// any non-append divergence (e.g. user-triggered regenerate). The
		// `startsWith` check covers the shrink case — a shorter string can't
		// start with a longer one.
		if (extensionsChanged || this.#blocks.length === 0 || !content.startsWith(this.#content)) {
			this.#extensions = extensions;
			this.#content = content;
			this.#blocks = parseBlocks(content, [...extensions]);
			return this.#blocks;
		}

		const stable = this.#blocks.slice(0, -1);
		const prevLast = this.#blocks[this.#blocks.length - 1];
		const newSuffix = content.slice(this.#content.length);
		const tailBlocks = parseBlocks(prevLast + newSuffix, [...extensions]);

		this.#content = content;
		this.#blocks = stable.length === 0 ? tailBlocks : [...stable, ...tailBlocks];
		return this.#blocks;
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
