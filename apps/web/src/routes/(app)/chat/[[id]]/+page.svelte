<script lang="ts">
	import { goto } from '$app/navigation';
	import { buildSuggestions } from '$lib/chat/suggestions.js';
	import WebPromptTools from '$lib/components/WebPromptTools.svelte';
	import { ChatPromptInput, MessageList } from '@eurora/chat';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as Empty from '@eurora/ui/components/empty/index';
	import { toast } from 'svelte-sonner';

	let { data } = $props();

	const chat = inject(CHAT_SERVICE);

	const threadId = $derived(data.threadId);

	$effect(() => {
		if (threadId) {
			chat.activeThreadId = threadId;
			chat.loadMessages(threadId);
		} else {
			chat.activeThreadId = undefined;
		}
	});

	$effect(() => {
		const newThread = chat.newThread;
		if (newThread) {
			chat.newThread = undefined;
			goto(`/chat/${newThread.id}`, { replaceState: true, keepFocus: true });
		}
	});

	function errMsg(e: unknown): string {
		return e instanceof Error ? e.message : String(e);
	}

	function handleCopy(content: string) {
		navigator.clipboard
			.writeText(content)
			.catch((e) => toast.error(`Failed to copy: ${errMsg(e)}`));
	}

	function handleSubmit(text: string) {
		chat.sendMessage(text).catch((e) => toast.error(errMsg(e)));
	}

	function handleEdit(messageId: string, newText: string) {
		chat.editMessage(messageId, newText).catch((e) => toast.error(errMsg(e)));
	}

	const suggestions = $derived(buildSuggestions(handleSubmit));
</script>

{#snippet emptyState()}
	<Empty.Root>
		<Empty.Header>
			<Empty.Title>How can I help you today?</Empty.Title>
		</Empty.Header>
	</Empty.Root>
{/snippet}

<div class="flex h-full flex-col overflow-hidden">
	<MessageList onCopy={handleCopy} onEdit={handleEdit} {emptyState} />
	<ChatPromptInput onSubmit={handleSubmit} {suggestions}>
		{#snippet tools()}
			<WebPromptTools />
		{/snippet}
	</ChatPromptInput>
</div>
