<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { MessageList, MessageGraph, ChatPromptInput } from '@eurora/chat';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import ListIcon from '@lucide/svelte/icons/list';
	import NetworkIcon from '@lucide/svelte/icons/network';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type { TimelineAppEvent } from '$lib/bindings/bindings.js';

	let { data } = $props();

	const taurpc = inject(TAURPC_SERVICE);
	const chatService = inject(CHAT_SERVICE);
	let latestTimelineItem = $state<TimelineAppEvent | null>(null);

	const threadId = $derived(data.threadId);
	const hasMessages = $derived((chatService.activeThread?.messages.length ?? 0) > 0);

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
		writeText(content).catch((e) => toast.error(`Failed to copy: ${e}`));
	}

	function handleSubmit(text: string) {
		chatService.sendMessage(text).catch((e) => toast.error(String(e)));
	}

	function handleEdit(messageId: string, newText: string) {
		chatService.editMessage(messageId, newText).catch((e) => toast.error(String(e)));
	}

	function handleGraphNavigate(messageId: string) {
		if (!threadId) return;
		chatService.switchBranch(threadId, messageId, 0).catch((e) => toast.error(String(e)));
		chatService.viewMode = 'list';
	}

	onMount(() => {
		taurpc.timeline.new_app_event.on((e) => {
			latestTimelineItem = e;
		});
	});

	const suggestions = [
		'What are the latest trends in AI?',
		'How does machine learning work?',
		'Explain quantum computing',
		'Best practices for React development',
		'Tell me about TypeScript benefits',
		'How to optimize database queries?',
		'What is the difference between SQL and NoSQL?',
		'Explain cloud computing basics',
	];
</script>

{#snippet emptyState()}
	<Empty.Root>
		<Empty.Header>
			{#if latestTimelineItem?.icon_base64}
				<Empty.Title>Currently on</Empty.Title>
				<Empty.Media variant="icon" class="bg-transparent">
					<img src={latestTimelineItem.icon_base64} alt="" class="size-full" />
				</Empty.Media>
			{:else}
				<Empty.Title>No messages yet</Empty.Title>
			{/if}
		</Empty.Header>
	</Empty.Root>
{/snippet}

<div class="flex h-full flex-col overflow-hidden">
	{#if hasMessages}
		<div class="flex justify-end px-4 py-2">
			<div class="bg-muted inline-flex rounded-md p-0.5">
				<Button
					variant={chatService.viewMode === 'list' ? 'secondary' : 'ghost'}
					size="sm"
					onclick={() => (chatService.viewMode = 'list')}
				>
					<ListIcon class="size-4" />
				</Button>
				<Button
					variant={chatService.viewMode === 'graph' ? 'secondary' : 'ghost'}
					size="sm"
					onclick={() => (chatService.viewMode = 'graph')}
				>
					<NetworkIcon class="size-4" />
				</Button>
			</div>
		</div>
	{/if}

	{#if chatService.viewMode === 'graph' && hasMessages}
		<MessageGraph onMessageDblClick={handleGraphNavigate} class="min-h-0 flex-1" />
	{:else}
		<MessageList onCopy={handleCopy} onEdit={handleEdit} {emptyState} />
		<ChatPromptInput onSubmit={handleSubmit} {suggestions} />
	{/if}
</div>
