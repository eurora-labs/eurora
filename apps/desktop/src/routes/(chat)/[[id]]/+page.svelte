<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { buildSuggestions } from '$lib/chat/suggestions.js';
	import { MessageList, MessageGraph, ChatPromptInput, middleTruncate } from '@eurora/chat';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as Attachment from '@eurora/ui/components/ai-elements/attachments/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type { ContextChip, TimelineAppEvent } from '$lib/bindings/bindings.js';

	let { data } = $props();

	const taurpc = inject(TAURPC_SERVICE);
	const chatService = inject(CHAT_SERVICE);
	let assets = $state<ContextChip[]>([]);
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
		chatService.sendMessage(text, assets).catch((e) => toast.error(String(e)));
	}

	function removeAsset(id: string) {
		assets = assets.filter((a) => a.id !== id);
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
		taurpc.timeline.new_assets_event.on((chips) => {
			assets = chips;
		});

		taurpc.timeline.new_app_event.on((e) => {
			latestTimelineItem = e;
		});
	});

	const suggestions = $derived(
		buildSuggestions({ chips: assets, chatService, send: handleSubmit }),
	);
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
	{#if chatService.viewMode === 'graph' && hasMessages}
		<MessageGraph onMessageDblClick={handleGraphNavigate} class="min-h-0 flex-1" />
	{:else}
		<MessageList onCopy={handleCopy} onEdit={handleEdit} {emptyState} />
	{/if}
	<ChatPromptInput onSubmit={handleSubmit} {suggestions}>
		{#snippet header()}
			{#if assets.length > 0}
				<Attachment.Root variant="inline">
					{#each assets as asset (asset.id)}
						<Attachment.Item
							data={{
								type: 'file',
								id: asset.id,
								filename: middleTruncate(asset.name),
							}}
							onRemove={() => removeAsset(asset.id)}
						>
							<Attachment.Preview />
							<Attachment.Info />
							<Attachment.Remove />
						</Attachment.Item>
					{/each}
				</Attachment.Root>
			{/if}
		{/snippet}
	</ChatPromptInput>
</div>
