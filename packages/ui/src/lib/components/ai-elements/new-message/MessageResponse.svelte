<script lang="ts">
	import { cn } from '$lib/utils';
	import { Streamdown, type StreamdownProps, type Extension } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import MathElement from 'svelte-streamdown/math';
	import { mode } from 'mode-watcher';
	import type { HTMLAttributes } from 'svelte/elements';

	import githubLightDefault from '@shikijs/themes/github-light-default';
	import githubDarkDefault from '@shikijs/themes/github-dark-default';

	type Props = {
		content: string;
		class?: string;
	} & Omit<StreamdownProps, 'content' | 'class'> &
		Omit<HTMLAttributes<HTMLDivElement>, 'content'>;

	let { content, class: className, ...restProps }: Props = $props();
	let currentTheme = $derived(
		mode.current === 'dark' ? 'github-dark-default' : 'github-light-default',
	);

	function ensureBlankLinesAroundMath(text: string): string {
		return text.replace(/(\$\$\n[\s\S]*?\n\$\$)/g, (match, _, offset, src) => {
			const before = src.substring(Math.max(0, offset - 2), offset);
			const afterIdx = offset + match.length;
			const after = src.substring(afterIdx, afterIdx + 2);

			let result = match;
			if (before.length > 0 && before !== '\n\n') result = '\n\n' + result;
			if (after.length > 0 && after !== '\n\n') result = result + '\n\n';
			return result;
		});
	}

	const normalizedContent = $derived(ensureBlankLinesAroundMath(content));

	const blockMathRule = /^\$\$[ \t]*\n?([\s\S]+?)\n?[ \t]*\$\$(?:\s|$)/;

	const blockMathExtension: Extension = {
		name: 'math',
		level: 'block',
		tokenizer(src) {
			const match = src.match(blockMathRule);
			if (match) {
				return {
					type: 'math',
					isInline: false,
					displayMode: true,
					raw: match[0],
					text: match[1].trim(),
				};
			}
		},
	};
</script>

<div class={cn('size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0', className)}>
	<Streamdown
		content={normalizedContent}
		shikiTheme={currentTheme}
		baseTheme="shadcn"
		components={{ code: Code, math: MathElement }}
		katexConfig={{ throwOnError: false }}
		extensions={[blockMathExtension]}
		shikiThemes={{
			'github-light-default': githubLightDefault,
			'github-dark-default': githubDarkDefault,
		}}
		{...restProps}
	/>
</div>
