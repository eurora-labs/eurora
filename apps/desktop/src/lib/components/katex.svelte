<script lang="ts" module>
	import rehypeKatex from 'rehype-katex';
	import rehypeSanitize, { defaultSchema } from 'rehype-sanitize';
	import rehypeStringify from 'rehype-stringify';
	import remarkGfm from 'remark-gfm';
	import remarkMath from 'remark-math';
	import remarkParse from 'remark-parse';
	import remarkRehype from 'remark-rehype';
	import { unified } from 'unified';

	const katexSchema = structuredClone(defaultSchema);

	const mathTags = [
		'math',
		'semantics',
		'mrow',
		'mi',
		'mo',
		'mn',
		'ms',
		'mtext',
		'mspace',
		'msup',
		'msub',
		'msubsup',
		'mfrac',
		'mover',
		'munder',
		'munderover',
		'msqrt',
		'mroot',
		'mtable',
		'mtr',
		'mtd',
		'menclose',
		'mpadded',
		'mphantom',
		'mglyph',
		'annotation',
		'annotation-xml',
	];

	katexSchema.tagNames = [...(katexSchema.tagNames ?? []), ...mathTags];
	katexSchema.attributes = {
		...katexSchema.attributes,
		div: [...(katexSchema.attributes?.['div'] ?? []), ['className', /^katex/]],
		span: [...(katexSchema.attributes?.['span'] ?? []), ['className', /^katex/], 'style'],
		math: ['xmlns', 'display'],
		annotation: ['encoding'],
	};

	const mathProcessor = unified()
		.use(remarkParse)
		.use(remarkGfm)
		.use(remarkMath)
		.use(remarkRehype)
		.use(rehypeKatex, { output: 'htmlAndMathml' })
		.use(rehypeSanitize, katexSchema)
		.use(rehypeStringify);

	const plainProcessor = unified()
		.use(remarkParse)
		.use(remarkGfm)
		.use(remarkRehype)
		.use(rehypeSanitize, katexSchema)
		.use(rehypeStringify);

	const mathPattern = /\$\$|\$[^$]|\\\[|\\\(|```(?:math|latex)\b/;
</script>

<script lang="ts">
	let renderGeneration = 0;

	async function renderKatex(elem: HTMLElement, math: string) {
		const generation = ++renderGeneration;

		try {
			const hasMath = mathPattern.test(math);
			math = normalizeMathDelimiters(math);
			const processor = hasMath ? mathProcessor : plainProcessor;
			const file = await processor.process(math);

			if (generation !== renderGeneration) return;
			elem.innerHTML = String(file);
		} catch (error) {
			if (generation !== renderGeneration) return;
			console.error('Failed to render KaTeX:', error);
		}
	}

	function normalizeMathDelimiters(input: string): string {
		return input
			.replace(/\\\[/g, '$$$$\n')
			.replace(/\\\]/g, '\n$$$$')
			.replace(/\\\(/g, '$$')
			.replace(/\\\)/g, '$$')
			.replace(/```(?:math|latex)\n([\s\S]*?)\n```/g, '$$$$\n$1\n$$$$');
	}

	let { math }: { math: string } = $props();

	let htmlElement: HTMLElement;

	$effect(() => {
		if (htmlElement && math) {
			renderKatex(htmlElement, math);
		}
	});
</script>

<div bind:this={htmlElement} class="katex-content"></div>

<style>
	/* shadcn/ui typography — applied to unified pipeline output */

	/* Headings */
	div :global(h1) {
		font-weight: 800;
		font-size: var(--text-4xl);
		line-height: var(--tw-leading, 1);
		letter-spacing: -0.025em;
		scroll-margin-top: 5rem;
	}

	div :global(h2) {
		margin-top: 2.5rem;
		padding-bottom: 0.5rem;
		border-bottom: 1px solid var(--color-border);
		font-weight: 600;
		font-size: var(--text-3xl);
		line-height: var(--tw-leading, 1);
		letter-spacing: -0.025em;
		scroll-margin-top: 5rem;
	}

	div :global(h2:first-child) {
		margin-top: 0;
	}

	div :global(h3) {
		margin-top: 2rem;
		font-weight: 600;
		font-size: var(--text-2xl);
		line-height: var(--tw-leading, 1);
		letter-spacing: -0.025em;
		scroll-margin-top: 5rem;
	}

	div :global(h4) {
		font-weight: 600;
		font-size: var(--text-xl);
		line-height: var(--tw-leading, 1);
		letter-spacing: -0.025em;
		scroll-margin-top: 5rem;
	}

	/* Paragraphs */
	div :global(p) {
		line-height: 1.75;
	}

	div :global(p:not(:first-child)) {
		margin-top: 1.5rem;
	}

	/* Links */
	div :global(a) {
		color: var(--color-primary);
		font-weight: 500;
		text-decoration: underline;
		text-underline-offset: 4px;
	}

	/* Blockquotes */
	div :global(blockquote) {
		margin-top: 1.5rem;
		padding-inline-start: 1.5rem;
		border-inline-start: 2px solid currentColor;
		font-style: italic;
	}

	/* Tables */
	div :global(table) {
		width: 100%;
	}

	div :global(thead + tbody),
	div :global(tr + tr) {
		border-top: 1px solid var(--color-border);
	}

	div :global(thead tr) {
		border-top: 1px solid var(--color-border);
	}

	div :global(tr:nth-child(even)) {
		background-color: var(--color-muted);
	}

	div :global(tr) {
		margin: 0;
		padding: 0;
	}

	div :global(th) {
		padding: 0.5rem 1rem;
		border: 1px solid var(--color-border);
		font-weight: 700;
		text-align: start;
	}

	div :global(td) {
		padding: 0.5rem 1rem;
		border: 1px solid var(--color-border);
		text-align: start;
	}

	/* Table wrapper for overflow */
	div :global(.my-6) {
		width: 100%;
		margin-block: 1.5rem;
		overflow-y: auto;
	}

	/* Lists */
	div :global(ul) {
		margin-inline-start: 1.5rem;
		margin-block: 1.5rem;
		list-style-type: disc;
	}

	div :global(ol) {
		margin-inline-start: 1.5rem;
		margin-block: 1.5rem;
		list-style-type: decimal;
	}

	div :global(li + li) {
		margin-top: 0.5rem;
	}

	/* Inline code */
	div :global(code) {
		position: relative;
		padding: 0.2rem 0.3rem;
		border-radius: 0.25rem;
		background-color: var(--color-muted);
		font-weight: 600;
		font-size: 0.875em;
		font-family: var(--font-mono, ui-monospace, monospace);
	}

	/* Code blocks — reset inline code styles on pre > code */
	div :global(pre) {
		margin-block: 1.5rem;
		padding: 1rem;
		overflow-x: auto;
		border-radius: 0.5rem;
		background-color: var(--color-muted);
	}

	div :global(pre code) {
		padding: 0;
		background-color: transparent;
		font-weight: normal;
		font-size: 0.875em;
	}

	/* Horizontal rules */
	div :global(hr) {
		margin-block: 1.5rem;
		border-color: var(--color-border);
	}

	/* KaTeX display math overflow */
	div :global(.katex-display) {
		overflow-x: auto;
		overflow-y: hidden;
	}
</style>
