<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/button/index.js';
	import Check from '@lucide/svelte/icons/check';
	import Copy from '@lucide/svelte/icons/copy';
	import { useEnvironmentVariable } from './environment-variables-context.svelte.js';

	let {
		class: className,
		oncopy: onCopyCallback,
		onerror: onErrorCallback,
		timeout = 2000,
		copyFormat = 'value',
		children,
		...restProps
	}: {
		class?: string;
		oncopy?: () => void;
		onerror?: (error: Error) => void;
		timeout?: number;
		copyFormat?: 'name' | 'value' | 'export';
		children?: Snippet;
	} & Record<string, unknown> = $props();

	const variable = useEnvironmentVariable();

	let isCopied = $state(false);
	let timeoutId: ReturnType<typeof setTimeout> | undefined;

	function getTextToCopy(): string {
		const formatMap = {
			export: () => `export ${variable.name}="${variable.value}"`,
			name: () => variable.name,
			value: () => variable.value,
		};
		return formatMap[copyFormat]();
	}

	async function copyToClipboard() {
		if (typeof window === 'undefined' || !navigator?.clipboard?.writeText) {
			onErrorCallback?.(new Error('Clipboard API not available'));
			return;
		}
		try {
			await navigator.clipboard.writeText(getTextToCopy());
			isCopied = true;
			onCopyCallback?.();
			clearTimeout(timeoutId);
			timeoutId = setTimeout(() => (isCopied = false), timeout);
		} catch (error) {
			onErrorCallback?.(error as Error);
		}
	}
</script>

<Button
	data-slot="environment-variable-copy-button"
	class={cn('size-6 shrink-0', className)}
	onclick={copyToClipboard}
	size="icon"
	variant="ghost"
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else if isCopied}
		<Check size={12} />
	{:else}
		<Copy size={12} />
	{/if}
</Button>
