<script lang="ts">
	import rehypeKatex from 'rehype-katex';
	import rehypeStringify from 'rehype-stringify';
	import remarkMath from 'remark-math';
	import remarkParse from 'remark-parse';
	import remarkRehype from 'remark-rehype';
	import { unified } from 'unified';

	async function renderKatex(elem: HTMLElement, math: string) {
		console.log('math', math);

		math = math.replace(/\\\[/g, '$').replace(/\\\]/g, '$');

		math = math.replace(/\\\(/g, '$$').replace(/\\\)/g, '$$');

		math = math.replace('```', '$$');

		console.log('changed math', math);

		const file = await unified()
			.use(remarkParse)
			.use(remarkMath)
			.use(remarkRehype)
			.use(rehypeKatex)
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

<span bind:this={htmlElement}>{math}</span>
