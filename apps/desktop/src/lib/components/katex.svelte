<script lang="ts">
	import rehypeKatex from 'rehype-katex';
	import rehypeStringify from 'rehype-stringify';
	import remarkGfm from 'remark-gfm';
	import remarkMath from 'remark-math';
	import remarkParse from 'remark-parse';
	import remarkRehype from 'remark-rehype';
	import { unified } from 'unified';

	const processor = unified()
		.use(remarkParse)
		.use(remarkGfm)
		.use(remarkMath)
		.use(remarkRehype)
		.use(rehypeKatex, { output: 'htmlAndMathml' })
		.use(rehypeStringify);

	let renderGeneration = 0;

	async function renderKatex(elem: HTMLElement, math: string) {
		const generation = ++renderGeneration;

		try {
			math = normalizeMathDelimiters(math);
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

<span bind:this={htmlElement}></span>
