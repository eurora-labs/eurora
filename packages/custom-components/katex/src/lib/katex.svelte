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

		console.log('changed math', math);

		const file = await unified()
			.use(remarkParse)
			.use(remarkMath, { singleDollarTextMath: false })
			.use(remarkRehype, { allowDangerousHtml: true })
			.use(rehypeRaw)
			.use(remarkGfm)
			.use(rehypeKatex, { output: 'htmlAndMathml' })
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

<span class="language-math" bind:this={htmlElement}>
	{math}
</span>

<!-- <span class="katex" bind:this={htmlElement}>{math}</span> -->
