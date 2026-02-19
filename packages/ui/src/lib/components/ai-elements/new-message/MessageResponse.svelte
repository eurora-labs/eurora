<script lang="ts">
	import { cn } from '$lib/utils';
	import { Streamdown, type StreamdownProps, type Extension } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import Math from 'svelte-streamdown/math';
	import { mode } from 'mode-watcher';
	import type { HTMLAttributes } from 'svelte/elements';

	import githubLightDefault from '@shikijs/themes/github-light-default';
	import githubDarkDefault from '@shikijs/themes/github-dark-default';

	const blockMathRule = /^(\$\$)(?:\n((?:\\[\s\S]|[^\\])+?)\n\1(?:\n|$)|([^$\n]+?)\1(?=\s|$|$))/;

	const mathBlockExtension: Extension = {
		name: 'math',
		level: 'block',
		applyInBlockParsing: true,
		tokenizer(src) {
			const match = src.match(blockMathRule);
			if (match) {
				const content = (match[2] || match[3]).trim();
				return {
					type: 'math',
					isInline: false,
					displayMode: true,
					raw: match[0],
					text: content,
				};
			}
		},
	};

	type Props = {
		content: string;
		class?: string;
	} & Omit<StreamdownProps, 'content' | 'class'> &
		Omit<HTMLAttributes<HTMLDivElement>, 'content'>;

	let { content, class: className, ...restProps }: Props = $props();
	let currentTheme = $derived(
		mode.current === 'dark' ? 'github-dark-default' : 'github-light-default',
	);
</script>

<div class={cn('size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0', className)}>
	<Streamdown
		{content}
		shikiTheme={currentTheme}
		baseTheme="shadcn"
		components={{ code: Code, math: Math }}
		extensions={[mathBlockExtension]}
		shikiThemes={{
			'github-light-default': githubLightDefault,
			'github-dark-default': githubDarkDefault,
		}}
		{...restProps}
	/>
</div>
