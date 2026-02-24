<script lang="ts">
	import type { BundledLanguage } from 'shiki';
	import { cn } from '$lib/utils.js';
	import {
		type TokenizedCode,
		highlightCode,
		createRawTokens,
		addKeysToTokens,
		isItalic,
		isBold,
		isUnderline,
		LINE_NUMBER_CLASSES,
	} from './highlighter.js';

	function toStyleString(style: Record<string, string> | undefined): string | undefined {
		if (!style) return undefined;
		return Object.entries(style)
			.map(([k, v]) => `${k}:${v}`)
			.join(';');
	}

	interface Props {
		code: string;
		language: BundledLanguage;
		showLineNumbers?: boolean;
		class?: string;
	}

	let { code, language, showLineNumbers = false, class: className }: Props = $props();

	let tokenized = $state<TokenizedCode>(highlightCode(code, language) ?? createRawTokens(code));

	$effect(() => {
		let cancelled = false;

		tokenized = highlightCode(code, language) ?? createRawTokens(code);

		highlightCode(code, language, (result) => {
			if (!cancelled) {
				tokenized = result;
			}
		});

		return () => {
			cancelled = true;
		};
	});

	let keyedLines = $derived(addKeysToTokens(tokenized.tokens));
</script>

<div data-slot="code-block-content" class="relative overflow-auto">
	<!--
		All whitespace inside <pre> is significant — every newline/tab between
		tags renders as a visible text node. Keep <pre>, <code>, and </code>,
		</pre> tightly collapsed to avoid phantom blank lines.
	-->
	<pre
		class={cn(
			'dark:!bg-[var(--shiki-dark-bg)] dark:!text-[var(--shiki-dark)] m-0 p-4 text-sm',
			className,
		)}
		style:background-color={tokenized.bg}
		style:color={tokenized.fg}><code
			class={cn(
				'font-mono text-sm',
				showLineNumbers && '[counter-increment:line_0] [counter-reset:line]',
			)}
			>{#each keyedLines as keyedLine (keyedLine.key)}<span
					class={showLineNumbers ? LINE_NUMBER_CLASSES : 'block'}
					>{#if keyedLine.tokens.length === 0}{'\n'}{:else}{#each keyedLine.tokens as { token, key } (key)}<span
								class="dark:!bg-[var(--shiki-dark-bg)] dark:!text-[var(--shiki-dark)]"
								style:background-color={token.bgColor}
								style:color={token.color}
								style:font-style={isItalic(token.fontStyle) ? 'italic' : undefined}
								style:font-weight={isBold(token.fontStyle) ? 'bold' : undefined}
								style:text-decoration={isUnderline(token.fontStyle)
									? 'underline'
									: undefined}
								style={toStyleString(token.htmlStyle)}>{token.content}</span
							>{/each}{/if}</span
				>{/each}</code
		></pre>
</div>
