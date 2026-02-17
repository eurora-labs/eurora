<script lang="ts">
	import rehypeKatex from 'rehype-katex';
	import rehypeRaw from 'rehype-raw';
	import rehypeStringify from 'rehype-stringify';
	import remarkGfm from 'remark-gfm';
	import remarkMath from 'remark-math';
	import remarkParse from 'remark-parse';
	import remarkRehype from 'remark-rehype';
	import { unified } from 'unified';

	async function renderKatex(elem: HTMLElement, math: string) {
		try {
			math = math
				.replace(/\\\[/g, '$$$')
				.replace(/\\\]/g, '$$$')
				.replace(/\\\(/g, '$$$')
				.replace(/\\\)/g, '$$$')
				.replace(/```math/g, '$$$')
				.replace(/```latex/g, '$$$')
				.replace(/```/g, '$$$');

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
		} catch (error) {
			console.error('Failed to render Katex:', error);
		} finally {
			finishRendering();
		}
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
