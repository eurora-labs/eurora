<script lang="ts" module>
	import type { Snippet } from 'svelte';
	import type { Tokens } from 'marked';
	import type { ThemedToken } from 'shiki/core';

	export interface StreamingCodeProps {
		token: Tokens.Code;
		id: string;
	}

	interface StreamdownThemeShape {
		code: {
			base: string;
			header: string;
			language: string;
			buttons: string;
			container: string;
			pre: string;
			line: string;
			skeleton: string;
		};
		components: { button: string };
	}

	interface StreamdownContextLike {
		shikiTheme: string;
		theme: StreamdownThemeShape;
		isMounted: boolean;
		animationBlockStyle: string | undefined;
		animationTextStyle: string | undefined;
		controls: { code: boolean };
		icons?: {
			copy?: Snippet;
			check?: Snippet;
			download?: Snippet;
		};
	}
</script>

<script lang="ts">
	import { getContext, onDestroy, untrack } from 'svelte';
	import { watch } from 'runed';
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import DownloadIcon from '@lucide/svelte/icons/download';

	import {
		getShikiWorkerClient,
		type ShikiWorkerClient,
	} from './shiki/shiki-worker-client.svelte.js';
	import { isLanguageSupported } from './shiki/languages.js';

	const { token, id }: StreamingCodeProps = $props();

	const streamdown = getContext<StreamdownContextLike>('streamdown');
	if (!streamdown) {
		throw new Error('StreamingCode must be rendered inside a Streamdown context');
	}

	// `last` holds the most recent fully-tokenized snapshot delivered by the
	// worker. The render path below shows ONLY this snapshot — never a mix of
	// themed body and unstyled tail. Visible code therefore lags actual
	// streamed text by one worker round-trip (typically <30ms once warm).
	// That's imperceptible while preserving full syntax colouring throughout
	// the stream.
	let last: { tokens: ThemedToken[][] } | null = $state(null);
	let copied = $state(false);
	let copyTimeout: number | null = null;

	let client: ShikiWorkerClient | null = null;
	try {
		client = getShikiWorkerClient();
	} catch (err) {
		console.warn('[streaming-code] worker unavailable, falling back to plaintext:', err);
	}

	// `id` is treated as stable for this component's lifetime — Streamdown
	// generates one per code block. Capture it once so a later prop change
	// can't cause us to release the wrong key on unmount.
	const requestKey = untrack(() => `streaming-code-${id}`);
	let requestVersion = 0;

	watch(
		() => [token.text, token.lang ?? '', streamdown.shikiTheme] as const,
		([code, lang, theme]) => {
			if (!client) return;
			if (!code) {
				last = null;
				return;
			}
			if (!isLanguageSupported(lang)) {
				// Unsupported language: rendered as plaintext below; don't waste
				// a worker round-trip producing the same plaintext tokens.
				last = null;
				return;
			}

			const myVersion = ++requestVersion;
			void client.request(requestKey, code, lang, theme).then((tokens) => {
				if (myVersion !== requestVersion) return;
				if (tokens === null) return;
				last = { tokens };
			});
		},
	);

	onDestroy(() => {
		client?.release(requestKey);
		if (copyTimeout !== null) {
			clearTimeout(copyTimeout);
			copyTimeout = null;
		}
	});

	// Pre-tokenization fallback: split the current text into lines for a
	// single render pass. Same monospace, same default colour as themed
	// tokens in the active Shiki theme — visually it just looks like code
	// that hasn't been syntax-highlighted yet, never like censored or muted
	// text.
	const plainLines = $derived((token.text ?? '').split('\n'));

	async function copy() {
		if (typeof navigator === 'undefined' || !navigator.clipboard?.writeText) {
			console.error('Clipboard API not available');
			return;
		}
		if (copied) return;
		try {
			await navigator.clipboard.writeText(token.text);
			copied = true;
			if (copyTimeout !== null) clearTimeout(copyTimeout);
			copyTimeout = window.setTimeout(() => {
				copied = false;
				copyTimeout = null;
			}, 2000);
		} catch (err) {
			console.error('Failed to copy to clipboard:', err);
		}
	}

	function download() {
		const ext = token.lang ? token.lang : 'txt';
		const filename = `file.${ext}`;
		const blob = new Blob([token.text], { type: 'text/plain' });
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = url;
		link.download = filename;
		document.body.appendChild(link);
		link.click();
		document.body.removeChild(link);
		URL.revokeObjectURL(url);
	}
</script>

<div
	data-streamdown-code={id}
	style={streamdown.isMounted ? streamdown.animationBlockStyle : ''}
	class={streamdown.theme.code.base}
>
	<div class={streamdown.theme.code.header}>
		<span class={streamdown.theme.code.language}>{token.lang}</span>
		{#if streamdown.controls.code}
			<div class={streamdown.theme.code.buttons}>
				<button
					class={streamdown.theme.components.button}
					onclick={download}
					title="Download code"
					type="button"
					aria-label="Download code"
				>
					{#if streamdown.icons?.download}
						{@render streamdown.icons.download()}
					{:else}
						<DownloadIcon size={16} />
					{/if}
				</button>
				<button
					class={streamdown.theme.components.button}
					onclick={copy}
					title="Copy code"
					type="button"
					aria-label={copied ? 'Copied' : 'Copy code'}
				>
					{#if copied}
						{#if streamdown.icons?.check}
							{@render streamdown.icons.check()}
						{:else}
							<CheckIcon size={16} />
						{/if}
					{:else if streamdown.icons?.copy}
						{@render streamdown.icons.copy()}
					{:else}
						<CopyIcon size={16} />
					{/if}
				</button>
			</div>
		{/if}
	</div>
	<div style="height: fit-content; width: 100%;" class={streamdown.theme.code.container}>
		{#if last}
			<pre class={streamdown.theme.code.pre}><code
					>{#each last.tokens as line}<span class={streamdown.theme.code.line}
							>{#each line as t}<span
									style={streamdown.isMounted
										? streamdown.animationTextStyle
										: ''}
									style:color={t.color}
									style:background-color={t.bgColor}>{t.content}</span
								>{/each}</span
						>{/each}</code
				></pre>
		{:else}
			<pre class={streamdown.theme.code.pre}><code
					>{#each plainLines as line}<span class={streamdown.theme.code.line}>{line}</span
						>{/each}</code
				></pre>
		{/if}
	</div>
</div>
