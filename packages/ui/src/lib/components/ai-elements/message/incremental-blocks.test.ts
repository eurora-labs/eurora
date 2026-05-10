import { describe, expect, it } from 'vitest';
import { parseBlocks } from 'svelte-streamdown';

import { IncrementalBlocks } from './incremental-blocks.svelte.js';

/**
 * Each test asserts the incremental result matches the canonical
 * `parseBlocks(fullContent)` output. That's the contract callers rely on:
 * the chunks rendered by per-block Streamdowns must match what a single
 * Streamdown would produce.
 */
describe('IncrementalBlocks', () => {
	it('returns the same blocks as a fresh parse for the initial content', () => {
		const content = '# Heading\n\nFirst paragraph.\n\nSecond paragraph.';
		const inc = new IncrementalBlocks();
		const out = inc.derive(content);
		expect([...out]).toEqual(parseBlocks(content, []));
	});

	it('preserves stable block string identities across appends', () => {
		const inc = new IncrementalBlocks();
		const a = inc.derive('# Heading\n\nFirst paragraph.\n\nSecond');
		const b = inc.derive('# Heading\n\nFirst paragraph.\n\nSecond paragraph.');
		// The first two blocks should be the *same string object*, so
		// downstream `$derived` and `#each` keying skip work.
		expect(b[0]).toBe(a[0]);
		expect(b[1]).toBe(a[1]);
	});

	it('matches a fresh parse when content grows incrementally', () => {
		const final = '# Title\n\nIntro paragraph.\n\n```rust\nfn main() {}\n```\n\nOutro.';
		const milestones = [
			'# Tit',
			'# Title\n\nIntro',
			'# Title\n\nIntro paragraph.\n\n```rust',
			'# Title\n\nIntro paragraph.\n\n```rust\nfn main() {}\n',
			'# Title\n\nIntro paragraph.\n\n```rust\nfn main() {}\n```\n\nOutro.',
		];
		const inc = new IncrementalBlocks();
		for (const step of milestones) {
			const out = inc.derive(step);
			expect([...out]).toEqual(parseBlocks(step, []));
		}
		const out = inc.derive(final);
		expect([...out]).toEqual(parseBlocks(final, []));
	});

	it('handles setext heading promotion correctly', () => {
		// Adding `===` below `Title` retroactively promotes Title to a
		// setext h1. Re-lexing (prev_last_block + new_suffix) covers this
		// because the previous last block is included in the re-lex.
		const inc = new IncrementalBlocks();
		inc.derive('Lead paragraph.\n\nTitle');
		const out = inc.derive('Lead paragraph.\n\nTitle\n===\n\nBody.');
		expect([...out]).toEqual(parseBlocks('Lead paragraph.\n\nTitle\n===\n\nBody.', []));
	});

	it('falls back to a full re-parse on content shrink', () => {
		const inc = new IncrementalBlocks();
		inc.derive('# A\n\n# B\n\n# C');
		const out = inc.derive('# A\n\n# B');
		expect([...out]).toEqual(parseBlocks('# A\n\n# B', []));
	});

	it('falls back to a full re-parse when the prefix diverges', () => {
		const inc = new IncrementalBlocks();
		inc.derive('# A\n\n# B\n\n# C');
		const out = inc.derive('# A\n\n# B\n\n# Z');
		expect([...out]).toEqual(parseBlocks('# A\n\n# B\n\n# Z', []));
	});

	it('returns the cached array when called with identical content', () => {
		const inc = new IncrementalBlocks();
		const a = inc.derive('# A\n\n# B');
		const b = inc.derive('# A\n\n# B');
		expect(b).toBe(a);
	});

	it('handles fenced code blocks that span chunks', () => {
		const inc = new IncrementalBlocks();
		const milestones = [
			'```rust\nfn main()',
			'```rust\nfn main() {\n    let x = 1;',
			'```rust\nfn main() {\n    let x = 1;\n}\n```',
			'```rust\nfn main() {\n    let x = 1;\n}\n```\n\nDone.',
		];
		for (const step of milestones) {
			const out = inc.derive(step);
			expect([...out]).toEqual(parseBlocks(step, []));
		}
	});

	it('does not duplicate the previous-last block when 2+ blocks are stable', () => {
		// Regression: `parseBlocks` drops blank-line separator tokens, so summed
		// block lengths fall short of `content.length`. Slicing `content` at
		// `sum(stable_lengths)` cuts inside the previous-last stable block and
		// re-parses its tail, producing a duplicate. Reproduces the user-visible
		// rendering bug where the trailing characters of a paragraph appear at
		// the start of the next one.
		const inc = new IncrementalBlocks();
		inc.derive('AA\n\nBB\n\nCC');
		const out = inc.derive('AA\n\nBB\n\nCC more');
		expect([...out]).toEqual(parseBlocks('AA\n\nBB\n\nCC more', []));
		expect(out).toHaveLength(3);
	});

	it('matches a fresh parse across a long multi-block streaming sequence', () => {
		// Mirrors the user-reported bug: a paragraph followed by another
		// paragraph followed by a new one — the third should not start with
		// the tail of the second.
		const final =
			'Distance from focal to circle edge = 1.82 * sqrt(2) ≈ 2.57 ' +
			'Ratio = 1.414 / 2.57 ≈ 0.55 ✓\n\n' +
			'Great, the math checks out.\n\n' +
			'Now, for the SIMD implementation:';

		const milestones: string[] = [];
		for (let i = 1; i <= final.length; i += 7) {
			milestones.push(final.slice(0, i));
		}
		milestones.push(final);

		const inc = new IncrementalBlocks();
		for (const step of milestones) {
			const out = inc.derive(step);
			expect([...out]).toEqual(parseBlocks(step, []));
		}
	});
});
