<script lang="ts">
	import rehypeKatex from 'rehype-katex';
	import rehypeStringify from 'rehype-stringify';
	import remarkMath from 'remark-math';
	import remarkParse from 'remark-parse';
	import remarkRehype from 'remark-rehype';
	import rehypeRaw from 'rehype-raw';
	import remarkGfm from 'remark-gfm';
	import { unified } from 'unified';

	async function renderKatex(elem: HTMLElement, math: string) {
		console.log('math', math);

		math = math.replace(/\\\[/g, '$$$').replace(/\\\]/g, '$$$');

		math = math.replace(/\\\(/g, '$$$').replace(/\\\)/g, '$$$');

		math = math.replace(/```math/g, '$$$');
		math = math.replace(/```latex/g, '$$$');

		math = math.replace(/```/g, '$$$');

		console.log('changed math', math);

		const file = await unified()
			.use(remarkParse)
			.use(remarkMath, { singleDollarTextMath: false })
			.use(remarkRehype, { allowDangerousHtml: true })
			.use(rehypeRaw)
			.use(remarkGfm)
			.use(rehypeKatex, { output: 'htmlAndMathml', displayMode: true } as any)
			.use(rehypeStringify)
			.process(math);

		elem.innerHTML = String(file);

		finishRendering();
	}

	let { finishRendering, math = $bindable() } = $props();

	let htmlElement: HTMLElement;

	$effect(() => {
		if (htmlElement && math) {
			renderKatex(htmlElement, math);
		}
	});
</script>

<span bind:this={htmlElement}>
	{math}
</span>

<!-- <span class="katex" bind:this={htmlElement}>{math}</span> -->
