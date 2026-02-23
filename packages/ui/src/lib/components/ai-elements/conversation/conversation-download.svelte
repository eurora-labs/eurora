<script lang="ts" module>
	export interface ConversationMessage {
		role: 'user' | 'assistant' | 'system' | 'data' | 'tool';
		content: string;
	}

	function defaultFormatMessage(message: ConversationMessage): string {
		const roleLabel = message.role.charAt(0).toUpperCase() + message.role.slice(1);
		return `**${roleLabel}:** ${message.content}`;
	}

	export function messagesToMarkdown(
		messages: ConversationMessage[],
		formatMessage: (
			message: ConversationMessage,
			index: number,
		) => string = defaultFormatMessage,
	): string {
		return messages.map((msg, i) => formatMessage(msg, i)).join('\n\n');
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';
	import Download from '@lucide/svelte/icons/download';

	let {
		messages,
		filename = 'conversation.md',
		formatMessage = defaultFormatMessage,
		class: className,
		children,
		...restProps
	}: Omit<ButtonProps, 'onclick'> & {
		messages: ConversationMessage[];
		filename?: string;
		formatMessage?: (message: ConversationMessage, index: number) => string;
		children?: Snippet;
	} = $props();

	function handleDownload() {
		const markdown = messagesToMarkdown(messages, formatMessage);
		const blob = new Blob([markdown], { type: 'text/markdown' });
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = url;
		link.download = filename;
		document.body.append(link);
		link.click();
		link.remove();
		URL.revokeObjectURL(url);
	}
</script>

<Button
	data-slot="conversation-download"
	class={cn(
		'absolute top-4 right-4 rounded-full dark:bg-background dark:hover:bg-muted',
		className,
	)}
	onclick={handleDownload}
	size="icon"
	type="button"
	variant="outline"
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<Download class="size-4" />
	{/if}
</Button>
