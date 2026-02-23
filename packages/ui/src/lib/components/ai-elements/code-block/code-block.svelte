<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { BundledLanguage } from 'shiki';
	import { cn } from '$lib/utils.js';
	import { CodeBlockState, setCodeBlockContext } from './code-block-context.svelte.js';
	import CodeBlockContainer from './code-block-container.svelte';
	import CodeBlockContent from './code-block-content.svelte';

	interface Props {
		code: string;
		language: BundledLanguage;
		showLineNumbers?: boolean;
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let {
		code,
		language,
		showLineNumbers = false,
		class: className,
		children,
		...rest
	}: Props = $props();

	let ctx = new CodeBlockState(code);
	setCodeBlockContext(ctx);

	$effect(() => {
		ctx.code = code;
	});
</script>

<CodeBlockContainer data-slot="code-block" {language} class={className} {...rest}>
	{@render children?.()}
	<CodeBlockContent {code} {language} {showLineNumbers} />
</CodeBlockContainer>
