<script lang="ts">
	import { goto } from '$app/navigation';
	import { MessageList, ChatPromptInput } from '@eurora/chat';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as Empty from '@eurora/ui/components/empty/index';
	import { toast } from 'svelte-sonner';

	let { data } = $props();

	const chatService = inject(CHAT_SERVICE);

	const threadId = $derived(data.threadId);

	$effect(() => {
		if (threadId) {
			chatService.activeThreadId = threadId;
			chatService.loadMessages(threadId);
		}
	});

	$effect(() => {
		const newThread = chatService.newThread;
		if (newThread) {
			chatService.newThread = undefined;
			goto(`/${newThread.id}`, { replaceState: true });
		}
	});

	function handleCopy(content: string) {
		navigator.clipboard.writeText(content).catch((e) => toast.error(`Failed to copy: ${e}`));
	}

	function handleSubmit(text: string) {
		chatService.sendMessage(text).catch((e) => toast.error(String(e)));
	}

	function handleEdit(messageId: string, newText: string) {
		chatService.editMessage(messageId, newText).catch((e) => toast.error(String(e)));
	}

	const suggestions = [
		'What are the latest trends in AI?',
		'How does machine learning work?',
		'Explain quantum computing',
		'Best practices for React development',
	];
</script>

{#snippet emptyState()}
	<Empty.Root>
		<Empty.Header>
			<Empty.Title>No messages yet</Empty.Title>
		</Empty.Header>
	</Empty.Root>
{/snippet}

<div class="flex h-full flex-col overflow-hidden">
	<MessageList onCopy={handleCopy} onEdit={handleEdit} {emptyState} />
	<ChatPromptInput onSubmit={handleSubmit} {suggestions} />
</div>
